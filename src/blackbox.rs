// Error reporting module

use core::fmt::Write;
use core::intrinsics;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;

use cortex_m_log::printer::Printer;

static mut LOG: MaybeUninit<crate::logging::T> = MaybeUninit::uninit();

pub fn init(log: crate::logging::T) -> &'static mut crate::logging::T {
    unsafe {
        LOG.as_mut_ptr().write(log);
        &mut *LOG.as_mut_ptr()
    }
}

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    let log = unsafe { &mut *LOG.as_mut_ptr() };
    let payload = panic_info.payload().downcast_ref::<&str>();
    match (panic_info.location(), payload) {
        (Some(location), Some(msg)) => {
            error!(
                log,
                "\r\npanic in file '{}' at line {}: {:?}\r\n",
                location.file(),
                location.line(),
                msg
            );
        }
        (Some(location), None) => {
            error!(
                log,
                "panic in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }
        (None, Some(msg)) => {
            error!(log, "panic: {:?}", msg);
        }
        (None, None) => {
            error!(log, "panic occured, no info available");
        }
    };
    unsafe { intrinsics::abort() }
}
