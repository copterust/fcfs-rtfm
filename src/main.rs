#![deny(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]

#[allow(unused)]
use panic_semihosting;

use cortex_m_semihosting::hprintln;
use rtfm::app;

// use ehal;
// use hal::delay::Delay;
// use hal::gpio::{self, AltFn, AF5, AF7};
// use hal::gpio::{LowSpeed, MediumSpeed, Output, PullNone, PullUp, PushPull};
use hal::gpio::PullUp;
use hal::prelude::*;
// use hal::serial::{self, Rx, Serial, Tx};
// use hal::spi::Spi;
// use hal::stm32f30x;
// use hal::timer;

#[app(device = stm32f30x)]
const APP: () = {
    static mut EXTI: stm32f30x::EXTI = ();

    #[init]
    fn init() {
        let device: stm32f30x::Peripherals = device;

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc.ahb);
        let _pa5 = gpioa.pa5.input().pull_type(PullUp);
        // this sohuld be properly done via HAL
        rcc.apb2.enr().write(|w| w.syscfgen().enabled());
        // Use PA0 as INT source
        // Set PA0 as EXTI0
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
        // Save device in resources for later use
        EXTI = device.EXTI;
    }

    #[interrupt(resources = [EXTI])]
    fn EXTI0() {
        let exti = resources.EXTI;
        exti.pr1.modify(|_, w| w.pr0().set_bit());
        hprintln!("EXTI0").unwrap();
    }
};
