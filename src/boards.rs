pub use crate::prelude::*;

pub struct BoardConfiguration<DPT,
 SPI,
 SpiPins,
 NPT,
 Usart,
 UsartPins,
 TxCh,
 GP,
 ExtiNum>
    where ExtiNum: hal::exti::ExternalInterrupt,
          GP: hal::gpio::GPIOPin
{
    pub debug_pin: DPT,
    pub spi: SPI,
    pub spi_pins: SpiPins,
    pub ncs: NPT,
    pub usart: Usart,
    pub usart_pins: UsartPins,
    pub tx_ch: TxCh,
    pub extih: hal::exti::BoundInterrupt<GP, ExtiNum>,
}

pub struct Peripherals {
    pub spi1: hal::pac::SPI1,
    pub spi2: hal::pac::SPI2,
    pub usart1: hal::pac::USART1,
    pub usart2: hal::pac::USART2,
    pub dma_channels: hal::dma::dma1::Channels,
    pub exti: hal::exti::ExternalInterrupts,
    pub clocks: hal::rcc::Clocks,
    pub gpioa: hal::gpio::Gpioa,
    pub gpiob: hal::gpio::Gpiob,
    pub gpioc: hal::gpio::Gpioc,
    pub syscfg: hal::syscfg::Syscfg,
}

#[cfg(configuration = "configuration_drone")]
mod defs {
    pub use super::Peripherals;
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PC15<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB9<PullNone, B>;
    type NT = NcsPinDef<Input>;
    pub type MpuIntPin = gpio::PC13<PullDown, Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA14<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI13;

    type Res = BoardConfiguration<DT,
                                  SpiT,
                                  SpiInputPins,
                                  NT,
                                  USART,
                                  UsartPins,
                                  TxCh,
                                  MpuIntPin,
                                  ExtiNum>;
    pub fn configure(mut device: Peripherals) -> Res {
        let scl_sck = device.gpiob.pb3;
        let ad0_sdo_miso = device.gpiob.pb4;
        let sda_sdi_mosi = device.gpiob.pb5;

        let mpu_interrupt_pin = device.gpioc.pc13.pull_type(PullDown);
        let extih =
            device.exti.EXTI13.bind(mpu_interrupt_pin, &mut device.syscfg);

        BoardConfiguration { debug_pin: device.gpioc.pc15,
                             spi: device.spi1,
                             spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                             ncs: device.gpiob.pb9,
                             usart: device.usart2,
                             usart_pins: (device.gpioa.pa14,
                                          device.gpioa.pa15),
                             tx_ch: device.dma_channels.7,
                             extih }
    }
}

#[cfg(configuration = "configuration_dev")]
mod defs {
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PA11<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB0<PullNone, B>;
    type NT = NcsPinDef<Input>;
    pub type MpuIntPin = gpio::PA0<PullDown, Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA2<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI0;

    type Res = BoardConfiguration<DT,
                                  SpiT,
                                  SpiInputPins,
                                  NT,
                                  USART,
                                  UsartPins,
                                  TxCh,
                                  MpuIntPin,
                                  ExtiNum>;
    pub fn configure(mut device: Peripherals) -> Res {
        let scl_sck = device.gpiob.pb3;
        let ad0_sdo_miso = device.gpiob.pb4;
        let sda_sdi_mosi = device.gpiob.pb5;

        let mpu_interrupt_pin = device.gpioa.pa0.pull_type(PullDown);
        let mut extih =
            device.exti.EXTI0.bind(mpu_interrupt_pin, &mut device.syscfg);

        BoardConfiguration { debug_pin: device.gpioa.pa11,
                             spi: device.spi1,
                             spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
                             ncs: device.gpiob.pb0,
                             usart: device.usart2,
                             usart_pins: (device.gpioa.pa2,
                                          device.gpioa.pa15),
                             tx_ch: device.dma_channels.7,
                             extih }
    }
}

pub use defs::*;

pub type SCLPin<B> = gpio::PB3<PullNone, B>;
pub type MISOPin<B> = gpio::PB4<PullNone, B>;
pub type MOSIPin<B> = gpio::PB5<PullNone, B>;
pub type SpiInputPins = (SCLPin<Input>, MISOPin<Input>, MOSIPin<Input>);

pub type SpiPins = (SCLPin<AltFn<AF5, PushPull, HighSpeed>>,
                    MISOPin<AltFn<AF5, PushPull, HighSpeed>>,
                    MOSIPin<AltFn<AF5, PushPull, HighSpeed>>);

pub type SPI = Spi<SpiT, SpiPins>;
pub type NcsPinT = NcsPinDef<Output<PushPull, HighSpeed>>;
pub type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
pub type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

pub type DebugPinT = DebugPinDef<PullNone, Output<PushPull, HighSpeed>>;

pub type QuadMotors = (gpio::PA0<PullNone, gpio::Input>,
                       gpio::PA1<PullNone, gpio::Input>,
                       gpio::PA2<PullNone, gpio::Input>,
                       gpio::PA3<PullNone, gpio::Input>);
pub type QuadMotorsTim = hal::pac::TIM2;

pub type Add2MotorsTim = hal::pac::TIM3;
pub type Add2Motors =
    (gpio::PA6<PullNone, gpio::Input>, gpio::PA7<PullNone, gpio::Input>);

pub mod mydevice {
    pub use super::Peripherals;
    use super::*;

    pub use hal::pac::NVIC_PRIO_BITS;

    impl Peripherals {
        pub fn steal() -> Self {
            let device = unsafe { hal::pac::Peripherals::steal() };
            let mut rcc = device.RCC.constrain();
            let gpioa = device.GPIOA.split(&mut rcc.ahb);
            let gpiob = device.GPIOB.split(&mut rcc.ahb);
            let gpioc = device.GPIOC.split(&mut rcc.ahb);
            let mut syscfg = device.SYSCFG.constrain(&mut rcc.apb2);
            let mut exti = device.EXTI.constrain();
            let dma_channels = device.DMA1.split(&mut rcc.ahb);
            let mut flash = device.FLASH.constrain();
            let clocks = rcc.cfgr
                            .sysclk(64.mhz())
                            .pclk1(32.mhz())
                            .pclk2(32.mhz())
                            .freeze(&mut flash.acr);

            Peripherals { spi1: device.SPI1,
                          spi2: device.SPI2,
                          usart1: device.USART1,
                          usart2: device.USART2,
                          dma_channels,
                          exti,
                          gpioa,
                          gpiob,
                          gpioc,
                          syscfg,
                          clocks }
        }
    }

    #[repr(u8)]
    #[derive(Clone, Copy)]
    #[allow(non_camel_case_types)]
    pub enum Interrupt {
        #[cfg(configuration = "configuration_drone")]
        MPU_EXT_INT = hal::pac::Interrupt::EXTI15_10 as u8,
        #[cfg(configuration = "configuration_dev")]
        MPU_EXT_INT = hal::pac::Interrupt::EXTI0 as u8,
    }

    unsafe impl rtfm::export::interrupt::Nr for Interrupt {
        fn nr(&self) -> u8 {
            *self as u8
        }
    }
}
