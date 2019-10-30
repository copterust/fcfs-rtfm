use crate::ahrs::AhrsResult;

pub struct State {
    pub ahrs: AhrsResult,
}

pub struct Control {
    telemetry: bool,
}

impl Control {
    #[inline]
    pub const fn new() -> Self {
        Control {
            telemetry: false
        }
    }

    #[inline]
    pub fn enable_telemetry(&mut self) {
        self.telemetry = true;
    }

    #[inline]
    pub fn disable_telemetry(&mut self) {
        self.telemetry = false;
    }

    #[inline]
    pub fn telemetry(&self) -> bool {
        self.telemetry
    }
}
