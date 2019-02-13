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
        // Use PA0 as INT source
        device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
        device.GPIOA.moder.modify(|_, w| w.moder0().input());
        device
            .GPIOA
            .pupdr
            .modify(|_, w| unsafe { w.pupdr0().bits(0b01) });
        // Set PCA0 as EXTI13
        device.RCC.apb2enr.write(|w| w.syscfgen().enabled());
        device
            .SYSCFG
            .exticr1
            .modify(|_, w| unsafe { w.exti0().bits(0b000) });
        // Enable external interrupt on rise
        device.EXTI.imr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr0().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr0().set_bit());
        hprintln!("init!").unwrap();
        // Save device in resources for later use
        DEVICE = device;
    }

    #[interrupt(resources = [DEVICE])]
    fn EXTI0() {
        hprintln!("EXTI0").unwrap();
    }
};
