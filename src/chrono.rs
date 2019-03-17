use asm_delay::CyclesToTime;
use hal::time::*;
use rtfm::Instant;

pub trait Chrono {
    type Time;
    /// Get the last measurements without updating state
    fn last(&self) -> Self::Time;

    /// Starts new cycle
    fn reset(&self) {
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
        RtfmClock {
            cc,
            last: Instant::now(),
        }
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
        self.last = now;
        self.cc.to_ms(duration.as_cycles())
    }
}
