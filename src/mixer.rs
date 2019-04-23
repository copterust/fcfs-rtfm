use nalgebra::{self, clamp};

use hal::gpio::Gpioa;
use hal::stm32f30x::Peripherals;
use hal::timer;

use crate::types::*;

pub existential type T: MotorCtrl;

pub fn create<F>(quad_tim: hal::stm32f30x::TIM2,
                 quad_pins: (gpio::PA0<PullNone, gpio::Input>,
                  gpio::PA1<PullNone, gpio::Input>,
                  gpio::PA2<PullNone, gpio::Input>,
                  gpio::PA3<PullNone, gpio::Input>),
                 plus2_tim: hal::stm32f30x::TIM3,
                 plus2_pins: (gpio::PA6<PullNone, gpio::Input>,
                  gpio::PA7<PullNone, gpio::Input>),
                 clocks: hal::rcc::Clocks,
                 freq: F)
                 -> T
    where F: Into<Hertz<u32>>
{
    let f = freq.into();
    // MOTORS:
    // pa0 -- pa3
    let (ch1, ch2, ch3, ch4, mut timer2) =
        timer::tim2::Timer::new(quad_tim, f, clocks).take_all();
    let mut m1_rear_right =
        quad_pins.0.pull_type(PullUp).to_pwm(ch1, MediumSpeed);
    let mut m2_front_right =
        quad_pins.1.pull_type(PullUp).to_pwm(ch2, MediumSpeed);
    let mut m3_rear_left =
        quad_pins.2.pull_type(PullUp).to_pwm(ch3, MediumSpeed);
    let mut m4_front_left =
        quad_pins.3.pull_type(PullUp).to_pwm(ch4, MediumSpeed);
    m1_rear_right.enable();
    m2_front_right.enable();
    m3_rear_left.enable();
    m4_front_left.enable();
    timer2.enable();

    let (ch5, ch6, _, _, mut timer3) =
        timer::tim3::Timer::new(plus2_tim, f, clocks).take_all();
    let mut m5_left = plus2_pins.0.pull_type(PullUp).to_pwm(ch5, MediumSpeed);
    let mut m6_right = plus2_pins.1.pull_type(PullUp).to_pwm(ch6, MediumSpeed);
    m5_left.enable();
    m6_right.enable();
    timer3.enable();
    #[cfg_attr(rustfmt, rustfmt_skip)]
    let map = Map6::from_row_slice(&[
        0.567, -0.815, -1.0, 1.0, /* rear left */
        0.567, 0.815, -1.0, 1.0, /* front right */
        -0.567, -0.815, 1.0, 1.0, /* rear left */
        -0.567, 0.815, 1.0, 1.0, /* front left */
        -1.0, -0.0, -1.0, 1.0, /* left */
        1.0, -0.0, 1.0, 1.0 /* right */
    ]);
    Mixer { map,
            max_duty: m1_rear_right.get_max_duty() as f32,
            pin: (m1_rear_right,
                  m2_front_right,
                  m3_rear_left,
                  m4_front_left,
                  m5_left,
                  m6_right) }
}

pub trait MotorCtrl {
    fn set_duty(&mut self, x: f32, y: f32, z: f32, thrust: f32);
}

pub struct Mixer<M, P> {
    pub map: M,
    pub pin: P,
    pub max_duty: f32,
}

pub type Ctrl = nalgebra::Vector4<f32>;
pub type Map4 = nalgebra::Matrix4<f32>;
pub type Map6 = nalgebra::Matrix6x4<f32>;

macro_rules! impl_motor_ctrl {
    ($map:ident, $num:expr, $($pin:ident $nr:tt)+) => (
        impl<$($pin),+> Mixer<$map, ($($pin),+)>
        where $($pin: ehal::PwmPin<Duty = u32>),+
        {
            // pub fn new(map: $map, pins: ($($pin),+), ) -> Self {
            //     Mixer {
            //     }
            // }

            pub fn get_duty(&mut self) -> [u32; $num] {
                [ $( self.pin.$nr.get_duty() ),+ ]
            }
        }

        impl<$($pin),+> MotorCtrl for Mixer<$map, ($($pin),+)>
        where $($pin: ehal::PwmPin<Duty = u32>),+ {
            fn set_duty(&mut self, x: f32, y: f32, z: f32, thrust: f32) {
                let duty = self.map * Ctrl::new(x, y, z, thrust);
                let max_duty = self.max_duty;
                $( self.pin.$nr.set_duty(clamp(duty[$nr], 0.0, max_duty) as u32); )+
            }
        }
    )
}

impl_motor_ctrl!(Map4, 4, A 0 B 1 C 2 D 3);
impl_motor_ctrl!(Map6, 6, A 0 B 1 C 2 D 3 E 4 F 5);
