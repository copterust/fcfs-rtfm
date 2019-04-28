#![deny(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![allow(unused)]
#![feature(core_intrinsics)]
#![feature(asm)]
#![feature(const_fn)]
#![feature(fn_traits, unboxed_closures)]
#![feature(impl_trait_in_bindings)]
#![feature(existential_type)]

use panic_abort;

mod ahrs;
mod chrono;
mod mixer;
#[macro_use]
mod logging;
mod telemetry;
mod types;

use core::fmt::Write;

use hal::delay::Delay;
use hal::prelude::*;

use asm_delay::{AsmDelay, CyclesToTime};
use cortex_m_log::printer::Printer;
use mpu9250::{Mpu9250, MpuConfig};
use nb::block;
use rtfm::{app, Instant};

use telemetry::Telemetry;
use types::*;

#[app(device = stm32f30x)]
const APP: () = {
    static mut EXTI0: hal::exti::Exti<hal::exti::EXTI0> = ();
    static mut AHRS: ahrs::AHRS<Dev, chrono::T> = ();
    static mut LOG: logging::T = ();
    static mut DEBUG_PIN: hal::gpio::PA1<PullDown,
                                           Output<PushPull, HighSpeed>> = ();
    // Option is needed to be able to change it in-flight (Option::take)
    static mut TELE: Option<telemetry::T> = ();

    #[init]
    fn init() -> init::LateResources {
        let freq = 72.mhz();
        let device: stm32f30x::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);
        // debug pin; TODO: change
        let pa1 =
            gpioa.pa1.output().output_speed(HighSpeed).pull_type(PullDown);

        // this should be properly done via HAL or rtfm vv
        // interrupt pin 3 purple -- a0
        // XXX: TODO: change pin
        let pa0 = gpioa.pa0.input().pull_type(PullDown);
        let mut syscfg = device.SYSCFG.constrain(&mut rcc.apb2);
        let mut exti = device.EXTI.constrain();
        exti.EXTI0.bind(pa0, &mut syscfg);

        let mut log = logging::create(core.ITM).unwrap();
        info!(log, "init!");
        let mut flash = device.FLASH.constrain();
        let clocks = rcc.cfgr
                        .sysclk(freq)
                        .pclk1(32.mhz())
                        .pclk2(32.mhz())
                        .freeze(&mut flash.acr);
        // SPI1
        let ncs = gpiob.pb0.output().push_pull();
        let scl_sck = gpiob.pb3;
        let sda_sdi_mosi = gpiob.pb5;
        let ad0_sdo_miso = gpiob.pb4;
        let spi = device.SPI1.spi((scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                                  mpu9250::MODE,
                                  1.mhz(),
                                  clocks);
        info!(log, "spi ok");
        let mut delay = AsmDelay::new(freq);
        info!(log, "delay ok");
        // MPU
        let gyro_rate = mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2);
        let mut mpu9250 =
            Mpu9250::imu_with_reinit(spi,
                                     ncs,
                                     &mut delay,
                                     &mut MpuConfig::imu().gyro_temp_data_rate(gyro_rate).sample_rate_divisor(3),
                                     |spi, ncs| {
                                         let (dev_spi, (scl, miso, mosi)) =
                                             spi.free();
                                         let new_spi =
                                             dev_spi.spi((scl, miso, mosi),
                                                         mpu9250::MODE,
                                                         20.mhz(),
                                                         clocks);
                                         Some((new_spi, ncs))
                                     }).unwrap();
        info!(log, "mpu ok");

        mpu9250.enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
               .unwrap();
        info!(log, "enabled; ");
        info!(log, "now: {:?}", mpu9250.get_enabled_interrupts());
        let mut ahrs = ahrs::AHRS::create_calibrated(mpu9250,
                                                     &mut delay,
                                                     chrono::rtfm_stopwatch(freq)).unwrap();
        info!(log, "ahrs ok");
        let mut usart2 =
            device.USART2.serial((gpioa.pa2, gpioa.pa15), Bps(460800), clocks);
        let (tx, _rx) = usart2.split();
        let channels = device.DMA1.split(&mut rcc.ahb);

        ahrs.setup_time();
        info!(log, "ready");
        init::LateResources { EXTI0: exti.EXTI0,
                              AHRS: ahrs,
                              TELE: Some(telemetry::create(channels.7, tx)),
                              LOG: log,
                              DEBUG_PIN: pa1 }
    }

    #[interrupt(binds=EXTI0,
                resources = [EXTI0, AHRS, LOG, DEBUG_PIN, TELE])]
    fn handle_mpu() {
        resources.DEBUG_PIN.set_high();
        let mut ahrs = resources.AHRS;
        let mut log = resources.LOG;
        let mut maybe_tele = resources.TELE.take();
        match ahrs.estimate() {
            Ok(result) => {
                // resources.TELE should always be Some, but for
                // future proof, let's be safe
                if let Some(tele) = maybe_tele {
                    let new_tele = tele.send(&result);
                    *resources.TELE = Some(new_tele);
                }

                debugfloats!(log,
                             ":",
                             result.ypr.yaw,
                             result.ypr.pitch,
                             result.ypr.roll);
            },
            Err(_e) => error!(log, "err"),
        };

        resources.DEBUG_PIN.set_low();
        resources.EXTI0.unpend();
    }
};
