pub use hal::dma::{self, dma1};
pub use hal::gpio::{self, AltFn, AF5};
pub use hal::gpio::{HighSpeed, LowSpeed, Output};
pub use hal::gpio::{PullDown, PullNone, PullUp, PushPull};
pub use hal::prelude::*;
pub use hal::serial::Tx;
pub use hal::spi::Spi;

pub type SPI = Spi<hal::stm32f30x::SPI1,
                   (gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                    gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                    gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>)>;
pub type Dev =
    mpu9250::SpiDevice<SPI, gpio::PB0<PullNone, Output<PushPull, LowSpeed>>>;
pub type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;
pub type USART = stm32f30x::USART2;
pub type TxUsart = Tx<USART>;
pub type TxCh = dma1::C7;
