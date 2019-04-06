use cortex_m_log::modes::InterruptOk;
use cortex_m_log::printer::dummy::Dummy;
use cortex_m_log::printer::itm::Itm;
use cortex_m_log::printer::semihosting::Semihosting;
use cortex_m_log::printer::Printer;
use ryu;

pub existential type T: Printer;

#[allow(unused)]
pub fn create(itm: cortex_m::peripheral::ITM) -> Result<T, ()> {
    #[cfg(log = "log_semihosting")]
    let log = Semihosting::<InterruptOk, _>::stdout();
    #[cfg(log = "log_dummy")]
    let log = Ok(Dummy);
    #[cfg(log = "log_itm")]
    let log = Itm::<InterruptOk>::new(itm);
    log
}

macro_rules! debug_guard {
    ($($args:tt)+) => {
        if cfg!(loglevel = "level_debug") {
            $($args)+;
        }
    }
}

macro_rules! info_guard {
    ($($args:tt)+) => {
        if cfg!(loglevel = "level_debug") || cfg!(loglevel = "level_info") {
            $($args)+;
        }
    }
}

macro_rules! debug {
    (
        $printer: expr,
        $($args:tt)+
    ) => {
        debug_guard!(write!($printer.destination(), $($args)+).unwrap())
    }
}

macro_rules! info {
    (
        $printer:expr,
        $($args:tt)+
    ) => {
        info_guard!(write!($printer.destination(), $($args)+).unwrap())
    }
}

macro_rules! error {
    (
        $printer:expr,
        $($args:tt)+
    ) => {
        write!($printer.destination(), $($args)+).unwrap()
    }
}

macro_rules! writelnfloats {
    (
        $w:expr,
        $prelude:expr,
        $($exprs:expr),* $(,)*
    ) => {
        {
            $w.write_str($prelude).unwrap();
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

macro_rules! infofloats {
    (
        $printer:expr,
        $prelude:expr,
        $($exprs:expr),* $(,)*
    ) => {
        info_guard!(writelnfloats!($printer.destination(), $prelude, $($exprs, )+));
    }
}

macro_rules! debugfloats {
    (
        $printer:expr,
        $prelude:expr,
        $($exprs:expr),* $(,)*
    ) => {
        debug_guard!(writelnfloats!($printer.destination(), $prelude, $($exprs, )+));
    }
}
