use crate::ahrs::AhrsResult;
use crate::prelude::*;

pub struct State {
    pub ahrs: AhrsResult,
    pub cmd: [f32; 3],
    pub errors: [f32; 3],
}

impl State {
    #[inline]
    pub const fn new() -> Self {
        State {
            ahrs: AhrsResult::new(),
            cmd: [0.0, 0.0, 0.0],
            errors: [0.0, 0.0, 0.0]
        }
    }
}

#[derive(Copy, Clone)]
pub struct Control {
    // permanent part
    pub telemetry: bool,
    pub pk: f32,
    pub ik: f32,
    pub dk: f32,
    pub pitch_pk: f32,
    pub roll_pk: f32,
    pub yaw_pk: f32,
    pub thrust: f32,
    // temp, as this has to be controlled from top level ctrl
    // XXX: units via naming? foooo...
    pub target_degrees: EulerAngles,
}

impl Control {
    #[inline]
    pub const fn new() -> Self {
        Control {
            telemetry: false,
            pk: 0.0,
            ik: 0.0,
            dk: 0.0,
            pitch_pk: 0.0,
            roll_pk: 0.0,
            yaw_pk: 0.0,
            thrust: 0.0,
            target_degrees: EulerAngles {
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0
            }
        }
    }

    #[inline]
    pub fn coefficients(&self) -> [f32; 6] {
        [self.pk, self.ik, self.dk,
         self.pitch_pk,
         self.roll_pk,
         self.yaw_pk]
    }
}

pub struct Requests {
    pub status: bool,
    // TODO: reset, boot
}

impl Requests {
    #[inline]
    pub const fn new() -> Self {
        Requests {
            status: false
        }
    }
}

impl Default for Requests {
    fn default() -> Self {
        Requests::new()
    }
}
