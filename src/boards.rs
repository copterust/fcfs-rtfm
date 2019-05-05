use crate::types::*;

pub existential type DebugPinT: ehal::digital::OutputPin;
pub existential type NcsPinT: ehal::digital::OutputPin;
pub existential type MpuIntPinT: gpio::GPIOPin + ehal::digital::InputPin;

pub struct BoardConfiguration<SPI, SpiPins, Usart, UsartPins, TxCh> {
    pub debug_pin: DebugPinT,
    pub mpu_interrupt_pin: MpuIntPinT,
    pub spi: SPI,
    pub spi_pins: SpiPins,
    pub ncs: NcsPinT,
    pub usart: Usart,
    pub usart_pins: UsartPins,
    pub tx_ch: TxCh,
}

// XXX: ugly, but device.FLASH.constrain() prevents us from using
//      hal::pac::Peripherals in `configure`.
pub struct InputDevice {
    pub SPI1: hal::pac::SPI1,
    pub SPI2: hal::pac::SPI2,
    pub USART1: hal::pac::USART1,
    pub USART2: hal::pac::USART2,
    pub DMA1: hal::pac::DMA1,
}

#[cfg(configuration = "configuration_drone")]
mod defs {}

#[cfg(configuration = "configuration_dev")]
mod defs {
    use super::*;

    pub type SpiT = hal::pac::SPI1;
    // XXX: this is ugly, we only want to define input types here...
    //      or maybe we want to setup everything here, instead...
    pub type SpiPins = (gpio::PB3<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                        gpio::PB4<PullNone, AltFn<AF5, PushPull, HighSpeed>>,
                        gpio::PB5<PullNone, AltFn<AF5, PushPull, HighSpeed>>);
    pub type SpiInputPins = (gpio::PB3<PullNone, Input>,
                             gpio::PB4<PullNone, Input>,
                             gpio::PB5<PullNone, Input>);
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA2<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type TxCh = hal::dma::dma1::C7;

    type Result =
        BoardConfiguration<SpiT, SpiInputPins, USART, UsartPins, TxCh>;
    pub fn configure(device: InputDevice,
                     gpioa: gpio::Gpioa,
                     gpiob: gpio::Gpiob,
                     ahb: &mut hal::rcc::AHB)
                     -> Result {
        // XXX: this is ugly, we mix selection of pins and configuration...
        let pa11: DebugPinT =
            gpioa.pa11.output().output_speed(HighSpeed).pull_type(PullDown);
        let pa0: MpuIntPinT = gpioa.pa0.input().pull_type(PullDown);
        let pb0: NcsPinT = gpiob.pb0.output().push_pull();

        let scl_sck = gpiob.pb3;
        let sda_sdi_mosi = gpiob.pb5;
        let ad0_sdo_miso = gpiob.pb4;
        let dma_channels = device.DMA1.split(ahb);

        BoardConfiguration { debug_pin: pa11,
                             mpu_interrupt_pin: pa0,
                             spi: device.SPI1,
                             spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                             ncs: pb0,
                             usart: device.USART2,
                             usart_pins: (gpioa.pa2, gpioa.pa15),
                             tx_ch: dma_channels.7 }
    }
}

pub use defs::*;
