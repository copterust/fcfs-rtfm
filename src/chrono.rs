use asm_delay::CyclesToTime;
use hal::time::*;
use rtfm::Instant;

pub existential type T: Chrono;

pub fn rtfm_stopwatch<F: Into<Hertz<u32>>>(f: F) -> T {
    DwtClock::new(CyclesToTime::new(f))
}

pub trait Chrono: Sized {
    type Time;
    /// Get the last measurements without updating state
    fn last(&self) -> Self::Time;

    /// Starts new cycle
    fn reset(&mut self) {
        self.split_time_ms();
    }

    /// Get elapsed time (ms) since last measurement and start new cycle
    fn split_time_ms(&mut self) -> f32;

    /// Get elapsed time (s) since last measurement and start new cycle
    fn split_time_s(&mut self) -> f32 {
        self.split_time_ms() / 1000.
    }
}

pub struct RtfmClock {
    cc: CyclesToTime,
    last: Instant,
}

impl RtfmClock {
    pub fn new(cc: CyclesToTime) -> Self {
        // let dwt =  unsafe { &(*cortex_m::peripheral::DWT::ptr()) };
        // let now:u32 = dwt.cyccnt.read();
        RtfmClock { cc, last: Instant::now() }
    }
}

impl Chrono for RtfmClock {
    type Time = Instant;

    fn last(&self) -> Self::Time {
        self.last
    }

    fn split_time_ms(&mut self) -> f32 {
        let now = Instant::now();
        let duration = now.duration_since(self.last);
        let duration = now - self.last;
        self.last = now;
        self.cc.to_ms(duration.as_cycles())
    }
}

pub struct DwtClock {
    cc: CyclesToTime,
    last: u32,
}

impl DwtClock {
    pub fn new(cc: CyclesToTime) -> Self {
        let dwt =  unsafe { &(*cortex_m::peripheral::DWT::ptr()) };
        DwtClock { cc, last: dwt.cyccnt.read() }
    }
}

impl Chrono for DwtClock {
    type Time = u32;

    fn last(&self) -> Self::Time {
        self.last
    }

    fn split_time_ms(&mut self) -> f32 {
        let dwt =  unsafe { &(*cortex_m::peripheral::DWT::ptr()) };
        let now:u32 = dwt.cyccnt.read();
        let duration = now - self.last;
        self.last = now;
        self.cc.to_ms(duration)
    }
}
