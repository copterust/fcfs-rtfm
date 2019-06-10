pub use crate::prelude::*;

pub struct BoardConfiguration<DPT,
 MIPT,
 SPI,
 SpiPins,
 NPT,
 Usart,
 UsartPins,
 TxCh,
 ExtiNum>
    where ExtiNum: hal::exti::ExternalInterrupt
{
    pub debug_pin: DPT,
    pub mpu_interrupt_pin: MIPT,
    pub spi: SPI,
    pub spi_pins: SpiPins,
    pub ncs: NPT,
    pub usart: Usart,
    pub usart_pins: UsartPins,
    pub tx_ch: TxCh,
    pub extih: hal::exti::Exti<ExtiNum>,
}

// XXX: ugly, but device.FLASH.constrain() prevents us from using
//      hal::pac::Peripherals in `configure`.
pub struct InputDevice {
    pub SPI1: hal::pac::SPI1,
    pub SPI2: hal::pac::SPI2,
    pub USART1: hal::pac::USART1,
    pub USART2: hal::pac::USART2,
    pub DMA1: hal::pac::DMA1,
    pub EXTI: hal::exti::ExternalInterrupts,
}

#[cfg(configuration = "configuration_drone")]
mod defs {
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PA11<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type MpuIntPinDef<A, B> = gpio::PC13<A, B>;
    type MT = MpuIntPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB9<PullNone, B>;
    type NT = NcsPinDef<Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type SCLPin<B> = gpio::PB3<PullNone, B>;
    pub type MISOPin<B> = gpio::PB4<PullNone, B>;
    pub type MOSIPin<B> = gpio::PB5<PullNone, B>;
    pub type SpiInputPins = (SCLPin<Input>, MISOPin<Input>, MOSIPin<Input>);
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA14<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI13;

    type Res = BoardConfiguration<DT,
                                  MT,
                                  SpiT,
                                  SpiInputPins,
                                  NT,
                                  USART,
                                  UsartPins,
                                  TxCh,
                                  ExtiNum>;
    pub fn configure(device: InputDevice,
                     gpioa: gpio::Gpioa,
                     gpiob: gpio::Gpiob,
                     gpioc: gpio::Gpioc,
                     ahb: &mut hal::rcc::AHB)
                     -> Res {
        let scl_sck = gpiob.pb3;
        let ad0_sdo_miso = gpiob.pb4;
        let sda_sdi_mosi = gpiob.pb5;
        let dma_channels = device.DMA1.split(ahb);

        BoardConfiguration { debug_pin: gpioa.pa11,
                             mpu_interrupt_pin: gpioc.pc13,
                             spi: device.SPI1,
                             spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                             ncs: gpiob.pb9,
                             usart: device.USART2,
                             usart_pins: (gpioa.pa14, gpioa.pa15),
                             tx_ch: dma_channels.7,
                             extih: device.EXTI.EXTI13 }
    }
}

#[cfg(configuration = "configuration_dev")]
mod defs {
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PA11<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type MpuIntPinDef<A, B> = gpio::PA0<A, B>;
    type MT = MpuIntPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB0<PullNone, B>;
    type NT = NcsPinDef<Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type SCLPin<B> = gpio::PB3<PullNone, B>;
    pub type MISOPin<B> = gpio::PB4<PullNone, B>;
    pub type MOSIPin<B> = gpio::PB5<PullNone, B>;
    pub type SpiInputPins = (SCLPin<Input>, MISOPin<Input>, MOSIPin<Input>);
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA2<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI0;

    type Res = BoardConfiguration<DT,
                                  MT,
                                  SpiT,
                                  SpiInputPins,
                                  NT,
                                  USART,
                                  UsartPins,
                                  TxCh,
                                  ExtiNum>;
    pub fn configure(device: InputDevice,
                     gpioa: gpio::Gpioa,
                     gpiob: gpio::Gpiob,
                     gpioc: gpio::Gpioc,
                     ahb: &mut hal::rcc::AHB)
                     -> Res {
        let scl_sck = gpiob.pb3;
        let ad0_sdo_miso = gpiob.pb4;
        let sda_sdi_mosi = gpiob.pb5;
        let dma_channels = device.DMA1.split(ahb);

        BoardConfiguration { debug_pin: gpioa.pa11,
                             mpu_interrupt_pin: gpioa.pa0,
                             spi: device.SPI1,
                             spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                             ncs: gpiob.pb0,
                             usart: device.USART2,
                             usart_pins: (gpioa.pa2, gpioa.pa15),
                             tx_ch: dma_channels.7,
                             extih: device.EXTI.EXTI0 }
    }
}

pub use defs::*;

pub type SpiPins = (SCLPin<AltFn<AF5, PushPull, HighSpeed>>,
                    MISOPin<AltFn<AF5, PushPull, HighSpeed>>,
                    MOSIPin<AltFn<AF5, PushPull, HighSpeed>>);

pub type SPI = Spi<SpiT, SpiPins>;
pub type NcsPinT = NcsPinDef<Output<PushPull, HighSpeed>>;
pub type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
pub type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

pub type DebugPinT = DebugPinDef<PullDown, Output<PushPull, HighSpeed>>;

pub type QuadMotors = (gpio::PA0<PullNone, gpio::Input>,
                       gpio::PA1<PullNone, gpio::Input>,
                       gpio::PA2<PullNone, gpio::Input>,
                       gpio::PA3<PullNone, gpio::Input>);
pub type QuadMotorsTim = hal::pac::TIM2;

pub type Add2MotorsTim = hal::pac::TIM3;
pub type Add2Motors =
    (gpio::PA6<PullNone, gpio::Input>, gpio::PA7<PullNone, gpio::Input>);
