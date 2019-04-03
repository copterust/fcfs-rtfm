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
mod logging;

use asm_delay::{AsmDelay, CyclesToTime};
use core::fmt::Write;
use hal::delay::Delay;
use hal::dma::{dma1, CircBuffer};
use hal::gpio::{self, AltFn, AF5};
use hal::gpio::{HighSpeed, LowSpeed, Output, PullNone, PushPull};
use hal::gpio::{PullDown, PullUp};
use hal::prelude::*;
use hal::serial::Tx;
use hal::spi::Spi;
use mpu9250::Mpu9250;
use nb::block;
use rtfm::{app, Instant};
use ryu;

type SPI = Spi<hal::stm32f30x::SPI1,
               (gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>)>;
type Dev =
    mpu9250::SpiDevice<SPI, gpio::PB0<PullNone, Output<PushPull, LowSpeed>>>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;
type USART = stm32f30x::USART2;

macro_rules! writefloats {
    (
        $w:expr,
        $prelude:expr,
        $($exprs:expr),* $(,)*
    ) => {
        {
            $w.write_str($prelude).unwrap();
            $w.write_char(':').unwrap();
            $(
                let mut b = ryu::Buffer::new();
                let s = b.format($exprs);
                $w.write_str(s).unwrap();
                $w.write_char(';').unwrap();
            )+
                $w.write_char('\n').unwrap();
        }
    }
}

#[app(device = stm32f30x)]
const APP: () = {
    static EXTI: stm32f30x::EXTI = ();
    static mut AHRS: ahrs::AHRS<Dev, chrono::T> = ();
    static mut LOG: logging::T = ();
    static mut TX: Tx<USART> = ();
    static mut DEBUG_PIN: hal::gpio::PA1<PullDown,
                                           Output<PushPull, HighSpeed>> = ();

    #[init]
    fn init() -> init::LateResources {
        let freq = 72.mhz();
        // interrupt pin 3 purple -- a0
        let device: stm32f30x::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);
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

        let mut log = logging::semihosting().unwrap();
        writeln!(log, "init!").unwrap();
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
        writeln!(log, "spi ok").unwrap();
        let mut delay = AsmDelay::new(freq);
        writeln!(log, "delay ok").unwrap();
        // MPU
        let mut mpu9250 =
            Mpu9250::imu_with_reinit(spi,
                                     ncs,
                                     &mut delay,
                                     &mut mpu9250::MpuConfig::imu().gyro_temp_data_rate(mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2)),
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
        writeln!(log, "mpu ok").unwrap();

        mpu9250.enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
               .unwrap();
        writeln!(log, "enabled; ").unwrap();
        writeln!(log, "now: {:?}", mpu9250.get_enabled_interrupts().unwrap());
        let mut ahrs = ahrs::AHRS::create_calibrated(mpu9250,
                                                     &mut delay,
                                                     chrono::rtfm_stopwatch(freq)).unwrap();
        writeln!(log, "ahrs ok").unwrap();
        let mut usart2 =
            device.USART2.serial((gpioa.pa2, gpioa.pa3),
                                 hal::time::Bps(460800),
                                 clocks);
        let (tx, _rx) = usart2.split();

        ahrs.setup_time();
        writeln!(log, "ready").unwrap();
        init::LateResources { EXTI: device.EXTI,
                              AHRS: ahrs,
                              TX: tx,
                              LOG: log,
                              DEBUG_PIN: pa1 }
    }

    #[interrupt(binds=EXTI0, resources = [EXTI, AHRS, TX, LOG, DEBUG_PIN])]
    fn handle_mpu() {
        resources.DEBUG_PIN.set_high();
        let exti = resources.EXTI;
        let mut ahrs = resources.AHRS;
        let mut tx = resources.TX;
        let mut log = resources.LOG;
        match ahrs.estimate() {
            Ok((dcm, _gyro, _dt_s)) => {
                let pitch = dcm.pitch;
                writefloats!(tx, "p:", pitch);
            },
            Err(_e) => writeln!(log, "err").unwrap(),
        };

        exti.pr1.modify(|_, w| w.pr0().set_bit());
        resources.DEBUG_PIN.set_low();
    }
};
