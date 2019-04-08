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
    static EXTI: stm32f30x::EXTI = ();
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
        // interrupt pin 3 purple -- a0
        let _pa0 = gpioa.pa0.input().pull_type(PullDown);
        let pa1 =
            gpioa.pa1.output().output_speed(HighSpeed).pull_type(PullDown);

        // this should be properly done via HAL or rtfm vv
        rcc.apb2.enr().write(|w| w.syscfgen().enabled());
        device.SYSCFG.exticr1.modify(|_, w| unsafe { w.exti0().bits(0b000) });
        // Enable external interrupt on rise
        device.EXTI.imr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr0().set_bit());
        // ^^ this should be done via HAL

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
                                     &mut MpuConfig::imu().gyro_temp_data_rate(gyro_rate),
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
            device.USART2.serial((gpioa.pa2, gpioa.pa3),
                                 hal::time::Bps(460800),
                                 clocks);
        let (tx, _rx) = usart2.split();
        let channels = device.DMA1.split(&mut rcc.ahb);

        ahrs.setup_time();
        info!(log, "ready");
        init::LateResources { EXTI: device.EXTI,
                              AHRS: ahrs,
                              TELE: Some(telemetry::create(channels.7, tx)),
                              LOG: log,
                              DEBUG_PIN: pa1 }
    }

    #[interrupt(binds=EXTI0,
                resources = [EXTI, AHRS, LOG, DEBUG_PIN, TELE])]
    fn handle_mpu() {
        resources.DEBUG_PIN.set_high();
        let exti = resources.EXTI;
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
        resources.EXTI.pr1.modify(|_, w| w.pr0().set_bit());
    }
};
