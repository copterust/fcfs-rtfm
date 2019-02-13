#![deny(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]

extern crate panic_semihosting;
use rtfm::app;
use cortex_m_semihosting::hprintln;

#[app(device = stm32f30x)]
const APP: () = {
    static mut DEVICE: stm32f30x::Peripherals = ();

    #[init]
    fn init() {
        let device: stm32f30x::Peripherals = device;
        // Use PA5 for debug LED
        device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
        device.GPIOA.moder.modify(|_, w| w.moder5().output());
        device.GPIOA.bsrr.write(|w| w.bs5().clear_bit());
        // Use PC13 as INT source
        device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
        device.GPIOC.moder.modify(|_, w| w.moder13().input());
        device
            .GPIOC
            .pupdr
            .modify(|_, w| unsafe { w.pupdr13().bits(0b01) });
        // Set PC13 as EXTI13
        device.RCC.apb2enr.write(|w| w.syscfgen().enabled());
        device
            .SYSCFG
            .exticr4
            .modify(|_, w| unsafe { w.exti13().bits(0b010) });
        // Enable external interrupt on rise
        device.EXTI.imr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr13().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr13().set_bit());
        hprintln!("init!").unwrap();
        // Save device in resources for later use
        DEVICE = device;
    }

    #[interrupt(resources = [DEVICE])]
    fn EXTI15_10() {
        // Turn on debug LED
        resources.DEVICE.GPIOA.bsrr.write(|w| w.bs5().set_bit());
    }
};
