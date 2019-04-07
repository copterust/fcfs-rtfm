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
mod utils;
#[macro_use]
mod logging;

use asm_delay::{AsmDelay, CyclesToTime};
use core::fmt::Write;
use cortex_m_log::printer::Printer;
use hal::delay::Delay;
use hal::dma::{self, dma1};
use hal::gpio::{self, AltFn, AF5};
use hal::gpio::{HighSpeed, LowSpeed, Output, PullNone, PushPull};
use hal::gpio::{PullDown, PullUp};
use hal::prelude::*;
use hal::serial::Tx;
use hal::spi::Spi;
use heapless::consts::*;
use heapless::Vec;
use mpu9250::{Mpu9250, MpuConfig};
use nb::block;
use rtfm::{app, Instant};

use utils::ActionState;

type SPI = Spi<hal::stm32f30x::SPI1,
               (gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>)>;
type Dev =
    mpu9250::SpiDevice<SPI, gpio::PB0<PullNone, Output<PushPull, LowSpeed>>>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;
type USART = stm32f30x::USART2;
type TxUsart = Tx<USART>;
type CH = dma1::C7;
type Buffer = Vec<u8, U42>;
type TxReady = (&'static mut Buffer, CH, TxUsart);
type TxBusy = dma::Transfer<dma::R, &'static mut Buffer, CH, TxUsart>;
static mut BUFFER: Buffer = Vec::new();

#[app(device = stm32f30x)]
const APP: () = {
    static EXTI: stm32f30x::EXTI = ();
    static mut AHRS: ahrs::AHRS<Dev, chrono::T> = ();
    static mut LOG: logging::T = ();
    static mut DEBUG_PIN: hal::gpio::PA1<PullDown,
                                           Output<PushPull, HighSpeed>> = ();
    // Option is needed to be able to change it in-flight
    static mut TELE: Option<ActionState<TxReady, TxBusy>> = ();

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
        // NOTE(unsafe): mutable static
        let tele = unsafe { ActionState::Ready((&mut BUFFER, channels.7, tx)) };
        init::LateResources { EXTI: device.EXTI,
                              AHRS: ahrs,
                              TELE: Some(tele),
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
        // NOTE(unwrap): resources.TELE is always Some
        let mut tele = resources.TELE.take().unwrap();
        match ahrs.estimate() {
            Ok(result) => {
                let new_tele = match tele {
                    ActionState::Ready((mut buffer, ch, tx)) => {
                        format_ahrs_result(&mut buffer, &result);
                        ActionState::MaybeBusy(tx.write_all(ch, buffer))
                    },
                    ActionState::MaybeBusy(transfer) => {
                        if transfer.is_done() {
                            let (buffer, ch, tx) = transfer.wait();
                            ActionState::Ready((buffer, ch, tx))
                        } else {
                            // not ready yet, skip tansfer
                            // XXX: alternatevely, we can allocate bigger buffer
                            //      and use its chunks.
                            ActionState::MaybeBusy(transfer)
                        }
                    },
                };
                *resources.TELE = Some(new_tele);
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

fn format_ahrs_result(buffer: &mut Buffer, result: &ahrs::AhrsResult) {
    buffer[0] = 0;
    let mut i = 1;
    // ax,ay,az,gx,gy,gz,dt_s,y,p,r
    for f in result.short_results().into_iter() {
        format_float(buffer, i, *f);
        i += 4;
    }
    buffer[41] = 0;
}

fn format_float(buffer: &mut [u8], offset: usize, f: f32) {
    let bits = f.to_bits();
    let bytes: [u8; 4] = unsafe { core::mem::transmute(bits) };
    let mut i = offset;
    for byte in bytes.iter() {
        buffer[i] = *byte;
        i += 1;
    }
}
