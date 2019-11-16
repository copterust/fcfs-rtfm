use crate::boards::*;
use hal::timer;

pub trait MotorCtrl {
    fn set_duty(&mut self, x: f32, y: f32, z: f32, thrust: f32);
}

impl MotorCtrl for () {
    // dummy
    fn set_duty(&mut self, x: f32, y: f32, z: f32, thrust: f32) {}
}

pub struct Mixer<M, P> {
    pub map: M,
    pub pin: P,
    pub max_duty: f32,
}

pub type Map4 = [[f32; 4]; 4];
pub type Map6 = [[f32; 4]; 6];

macro_rules! impl_motor_ctrl {
    ($map:ident, $num:expr, $($pin:ident $nr:tt)+) => (
        impl<$($pin),+> Mixer<$map, ($($pin),+)>
        where $($pin: ehal::PwmPin<Duty = u32>),+
        {
            pub fn get_duty(&mut self) -> [u32; $num] {
                [ $( self.pin.$nr.get_duty() ),+ ]
            }
        }

        impl<$($pin),+> MotorCtrl for Mixer<$map, ($($pin),+)>
        where $($pin: ehal::PwmPin<Duty = u32>),+ {
            fn set_duty(&mut self, x: f32, y: f32, z: f32, thrust: f32) {
                // let duty = self.map * Ctrl::new(x, y, z, thrust);
                let max_duty = self.max_duty;
                $(
                    {
                        let row = self.map[$nr];
                        let iduty = row[0] * x + row[1] * y + row[2] * z + row[3] * thrust;
                        self.pin.$nr.set_duty(clamp(iduty, 0.0, max_duty) as u32);
                    }
                )+
            }
        }
    )
}

impl_motor_ctrl!(Map4, 4, A 0 B 1 C 2 D 3);
impl_motor_ctrl!(Map6, 6, A 0 B 1 C 2 D 3 E 4 F 5);

#[inline]
fn clamp<T: PartialOrd>(val: T, min: T, max: T) -> T {
    if val > min {
        if val < max {
            val
        } else {
            max
        }
    } else {
        min
    }
}
