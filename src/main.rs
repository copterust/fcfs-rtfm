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

use cortex_m_semihosting::{hprint, hprintln};
use rtfm::{app, Instant};
use ryu;

// use ehal;
use asm_delay::{AsmDelay, CyclesToTime};
use hal::delay::Delay;
use hal::gpio::PullDown;
use hal::gpio::{self, AltFn, AF5};
use hal::gpio::{HighSpeed, LowSpeed, Output, PullNone, PushPull};
use hal::prelude::*;
use hal::serial::Tx;
use hal::spi::Spi;
use mpu9250::Mpu9250;

type SPI = Spi<hal::stm32f30x::SPI1,
               (gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>)>;
type Dev =
    mpu9250::SpiDevice<SPI, gpio::PB0<PullNone, Output<PushPull, LowSpeed>>>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;
type USART = stm32f30x::USART2;

macro_rules! hwrite_floats {
    (
        $prelude:expr,
        $($exprs:expr),* $(,)*
    ) => {
        {
            hprint!($prelude).unwrap();
            hprint!(":").unwrap();
            $(
                let mut b = ryu::Buffer::new();
                let s = b.format($exprs);
                hprint!(s).unwrap();
                hprint!(";");
            )+
                hprint!("\n");
        }
    }
}

#[app(device = stm32f30x)]
const APP: () = {
    static mut EXTI: stm32f30x::EXTI = ();
    static mut AHRS: ahrs::AHRS<Dev, chrono::T> = ();
    static mut TX: Tx<USART> = ();

    #[init]
    fn init() -> init::LateResources {
        let freq = 72.mhz();
        // interrupt pin 3 purple -- a0
        let device: stm32f30x::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);
        let _pa0 = gpioa.pa0.input().pull_type(PullDown);

        // this should be properly done via HAL vv
        rcc.apb2.enr().write(|w| w.syscfgen().enabled());
        device.SYSCFG.exticr1.modify(|_, w| unsafe { w.exti0().bits(0b000) });
        // Enable external interrupt on rise
        device.EXTI.imr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr0().set_bit());
        // ^^ this should be done via HAL

        hprintln!("init!").unwrap();
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
        hprintln!("spi ok").unwrap();
        let mut delay = AsmDelay::new(freq);
        hprintln!("delay ok").unwrap();
        // MPU
        let mut mpu9250 =
            Mpu9250::imu_with_reinit(spi,
                                     ncs,
                                     &mut delay,
                                     &mut mpu9250::MpuConfig::imu(),
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
        hprintln!("mpu ok").unwrap();

        mpu9250.enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
               .unwrap();
        hprintln!("enabled; ").unwrap();
        hprintln!("now: {:?}", mpu9250.get_enabled_interrupts().unwrap());
        let mut ahrs = ahrs::AHRS::create_calibrated(mpu9250,
                                                   &mut delay,
                                                     chrono::rtfm_stopwatch(freq)).unwrap();
        hprintln!("ahrs ok").unwrap();
        let mut usart2 =
            device.USART2.serial((gpioa.pa2, gpioa.pa3),
                                 hal::time::Bps(460800),
                                 clocks);
        let (tx, _rx) = usart2.split();

        ahrs.setup_time();
        init::LateResources { EXTI: device.EXTI, AHRS: ahrs, TX: tx }
    }

    #[interrupt(resources = [EXTI, AHRS, TX])]
    fn EXTI0() {
        let exti = resources.EXTI;
        let mut ahrs = resources.AHRS;
        let mut tx = resources.TX;
        match ahrs.estimate() {
            Ok((dcm, _gyro, _dt_s)) => {
                let pitch = dcm.pitch;
                let pitch_bits = pitch.to_bits();
                let bytes: [u8; 4] =
                    unsafe { core::mem::transmute(pitch_bits) };
                for byte in bytes.iter() {
                    tx.write(*byte);
                }
            },
            Err(_e) => hprintln!("err").unwrap(),
        };
        // unpend?
        exti.pr1.modify(|_, w| w.pr0().set_bit());
    }
};
