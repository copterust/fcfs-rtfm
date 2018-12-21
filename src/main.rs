#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use rtfm::app;

#[app(device = stm32f30x)]
const APP: () = {
    #[init]
    fn init() {
        // Cortex-M peripherals
        let _core: rtfm::Peripherals = core;

        // Device specific peripherals
        let _device: stm32f30x::Peripherals = device;
    }
};
