#![allow(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]

#[allow(unused)]
use panic_abort;

use cortex_m_semihosting::{hprint, hprintln};
use rtfm::{app, Instant};
use ryu;

// use ehal;
use asm_delay::AsmDelay;
use hal::delay::Delay;
use hal::gpio::PullDown;
use hal::gpio::{self, AltFn, AF5};
use hal::gpio::{HighSpeed, LowSpeed, Output, PullNone, PushPull};
use hal::prelude::*;
use hal::spi::Spi;
use mpu9250::Mpu9250;

type SPI = Spi<
    hal::stm32f30x::SPI1,
    (
        gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
        gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
        gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
    ),
>;
type Dev = mpu9250::SpiDevice<SPI, gpio::PB0<PullNone, Output<PushPull, LowSpeed>>>;
type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

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
    static mut MPU: MPU9250 = ();

    #[init]
    fn init() -> init::LateResources {
        let freq = 72.mhz();
        // interrupt pin 3 purple -- a0
        let device: stm32f30x::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let gpiob = device.GPIOB.split(&mut rcc.ahb);
        let _pa0 = gpioa.pa0.input().pull_type(PullDown);

        // this should be properly done via HAL
        rcc.apb2.enr().write(|w| w.syscfgen().enabled());
        device
            .SYSCFG
            .exticr1
            .modify(|_, w| unsafe { w.exti0().bits(0b000) });
        // Enable external interrupt on rise
        device.EXTI.imr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr0().set_bit());
        // ^^ this should be done via HAL

        hprintln!("init!").unwrap();
        let mut flash = device.FLASH.constrain();
        let clocks = rcc
            .cfgr
            .sysclk(freq)
            .pclk1(32.mhz())
            .pclk2(32.mhz())
            .freeze(&mut flash.acr);
        // SPI1
        let ncs = gpiob.pb0.output().push_pull();
        let scl_sck = gpiob.pb3;
        let sda_sdi_mosi = gpiob.pb5;
        let ad0_sdo_miso = gpiob.pb4;
        let spi = device.SPI1.spi(
            (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
            mpu9250::MODE,
            1.mhz(),
            clocks,
        );
        hprintln!("spi ok").unwrap();
        let mut delay = AsmDelay::new(freq);
        hprintln!("delay ok").unwrap();
        // MPU
        let mut mpu9250 = Mpu9250::imu_default(spi, ncs, &mut delay).unwrap();
        hprintln!("mpu ok").unwrap();

        mpu9250
            .enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
            .unwrap();
        hprintln!("enabled; ").unwrap();
        hprintln!("now: {:?}", mpu9250.get_enabled_interrupts().unwrap());

        // Save device in resources for later use
        init::LateResources {
            EXTI: device.EXTI,
            MPU: mpu9250,
        }
    }

    #[interrupt(resources = [EXTI, MPU])]
    fn EXTI0() {
        let exti = resources.EXTI;
        let mpu = resources.MPU;
        let status = mpu
            .get_interrupt_status()
            .unwrap_or(mpu9250::InterruptEnable::empty());
        hprintln!("EXTI0: {:?}; now: {:?}", status, Instant::now()).unwrap();
        match mpu.all() {
            Ok(m) => {
                let a = m.accel;
                let g = m.gyro;
                hwrite_floats!("m:", a.x, a.y, a.z, g.x, g.y, g.z, m.temp);
            }
            Err(_e) => hprintln!("err").unwrap(),
        };
        // unpend?
        exti.pr1.modify(|_, w| w.pr0().set_bit());
    }
};
