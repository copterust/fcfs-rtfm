pub use crate::boards::*;
pub use crate::prelude::*;

pub type SPI = Spi<SpiT, SpiPins>;
pub type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
pub type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

pub type QuadMotors = (gpio::PA0<PullNone, gpio::Input>,
                       gpio::PA1<PullNone, gpio::Input>,
                       gpio::PA2<PullNone, gpio::Input>,
                       gpio::PA3<PullNone, gpio::Input>);
pub type QuadMotorsTim = hal::pac::TIM2;

pub type Add2MotorsTim = hal::pac::TIM3;
pub type Add2Motors =
    (gpio::PA6<PullNone, gpio::Input>, gpio::PA7<PullNone, gpio::Input>);
