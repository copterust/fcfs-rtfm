

pub trait Bootloader {
    fn check_request(&mut self);
    fn to_bootloader(&mut self);
    fn system_reset(&mut self);
}

// pub type T = impl Bootloader;

// #[inline]
// pub const fn create() -> impl Bootloader {
//     stm32f30x::Bootloader::new()
// }

pub mod stm32f30x {
    use core::arch::asm;

    use cortex_m::peripheral::SCB;
    use cortex_m::{self, interrupt, register::msp};
    use hal::pac::{PWR, RCC, RTC};

    use super::Bootloader as BootloaderTrait;

    const BOOTLOADER_REQUEST: u32 = 93;
    const STM32_RESET_FN_ADDRESS: u32 = 0x1FFFD804u32;
    const STM32_BOOTLOADER_ADDRESS: u32 = 0x1FFFD800;

    pub struct Bootloader;

    impl Bootloader {
        #[inline]
        pub const fn new() -> Self {
            Bootloader {}
        }

        fn enable_bkp(&mut self) {
            let rcc = unsafe { &*RCC::ptr() };
            let pwr = unsafe { &*PWR::ptr() };
            let rtc = unsafe { &*RTC::ptr() };
            // enable bkp registers
            (*rcc).apb1enr.modify(|r, w| w.pwren().bit(true));
            // clear data protection
            (*pwr).cr.modify(|r, w| w.dbp().bit(true));
        }
    }

    impl BootloaderTrait for Bootloader {
        fn check_request(&mut self) {
            self.enable_bkp();
            let rtc = unsafe { &*RTC::ptr() };
            let bkp0r = &(*rtc).bkpr[0];
            if bkp0r.read().bits() == BOOTLOADER_REQUEST {
                cortex_m::asm::dsb();
                unsafe {
                    asm!(
                        "cpsie i",
                        "movw r0, 0xd800",
                        "movt r0, 0x1fff",
                        "ldr r0, [r0]",
                        "msr MSP, r0",
                        inout("r0") 4 => _, // clobber
                        options(nostack)
                    );
                    let f = 0x1FFFD804u32 as *const fn();
                    (*f)();
                }
                cortex_m::asm::dsb();
                loop {
                    cortex_m::asm::nop(); // avoid rust-lang/rust#28728
                }
            }
        }

        fn to_bootloader(&mut self) {
            self.enable_bkp();
            let rtc = unsafe { &*RTC::ptr() };
            // write cookie to backup register and reset
            (*rtc).bkpr[0].write(|w| unsafe { w.bits(BOOTLOADER_REQUEST) });
            cortex_m::asm::dsb();
            self.system_reset();
        }

        fn system_reset(&mut self) {
            cortex_m::peripheral::SCB::sys_reset();
        }
    }
}
