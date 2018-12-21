#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use rtfm::app;

#[app(device = stm32f30x)]
const APP: () = {
    static mut DEVICE: stm32f30x::Peripherals = ();
    #[init]
    fn init() {
        let mut _core: rtfm::Peripherals = core;

        let device: stm32f30x::Peripherals = device;

        device.RCC.ahbenr.modify(|_, w| w.iopaen().set_bit());
        device.GPIOA.moder.modify(
            |_, w| w.moder5().output()
        );
        device.GPIOA.bsrr.write(|w| w.bs5().clear_bit());

        device.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
        device.GPIOC.moder.modify(
            |_, w| w.moder12().input()
        );
        device.GPIOC.pupdr.modify(|_, w| unsafe {
            w.pupdr12().bits(0b01)
        });

        device.SYSCFG.exticr4.modify(|_, w| unsafe {
            w.exti12().bits(0b010)
        });

        device.EXTI.imr1.modify(|_, w| w.mr12().set_bit());
        device.EXTI.emr1.modify(|_, w| w.mr12().set_bit());
        device.EXTI.rtsr1.modify(|_, w| w.tr12().set_bit());

        DEVICE = device;
    }

    #[interrupt(resources = [DEVICE])]
    fn EXTI15_10() {
        resources.DEVICE.GPIOA.bsrr.write(|w| w.bs5().set_bit());
    }
};
