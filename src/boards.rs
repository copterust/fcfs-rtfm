pub use crate::prelude::*;

pub struct BoardConfiguration<
    DPT,
    SPI,
    SpiPins,
    NPT,
    Usart,
    UsartPins,
    TxCh,
    GP,
    ExtiNum,
    MotorPins,
    MotorAux,
> where
    ExtiNum: hal::exti::ExternalInterrupt,
    GP: hal::gpio::GPIOPin,
{
    pub debug_pin: DPT,
    pub spi: SPI,
    pub spi_pins: SpiPins,
    pub ncs: NPT,
    pub usart: Usart,
    pub usart_pins: UsartPins,
    pub tx_ch: TxCh,
    pub extih: hal::exti::BoundInterrupt<GP, ExtiNum>,
    pub motor_pins: MotorPins,
    pub motor_aux: MotorAux,
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
    pub tim2: hal::pac::TIM2,
    pub tim3: hal::pac::TIM3,
}

pub type Motors = impl crate::mixer::MotorCtrl;

macro_rules! pwm {
    ($pin: expr,
     $ch: expr
    ) => {{
        let mut p = $pin.pull_type(PullUp).to_pwm($ch, MediumSpeed);
        p.enable();
        p
    }};
}

#[cfg(configuration = "configuration_drone")]
mod defs {
    pub use super::Peripherals;
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PC15<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB9<PullNone, B>;
    type NT = NcsPinDef<Input>;
    pub type MpuIntPin = gpio::PC13<PullUp, Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA14<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type RxUsart = Rx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI13;
    pub type MotorPins = (
        gpio::PA0<PullNone, gpio::Input>,
        gpio::PA1<PullNone, gpio::Input>,
        gpio::PA2<PullNone, gpio::Input>,
        gpio::PA3<PullNone, gpio::Input>,
        gpio::PA6<PullNone, gpio::Input>,
        gpio::PA7<PullNone, gpio::Input>,
    );
    pub type MotorAux = (hal::pac::TIM2, hal::pac::TIM3);

    type Res = BoardConfiguration<
        DT,
        SpiT,
        SpiInputPins,
        NT,
        USART,
        UsartPins,
        TxCh,
        MpuIntPin,
        ExtiNum,
        MotorPins,
        MotorAux,
    >;
    pub fn configure(mut device: Peripherals) -> Res {
        let scl_sck = device.gpiob.pb3;
        let ad0_sdo_miso = device.gpiob.pb4;
        let sda_sdi_mosi = device.gpiob.pb5;

        let mpu_interrupt_pin = device.gpioc.pc13.pull_type(PullUp);
        let extih = device
            .exti
            .EXTI13
            .bind(mpu_interrupt_pin, &mut device.syscfg);

        let motor_pins = (
            device.gpioa.pa0,
            device.gpioa.pa1,
            device.gpioa.pa2,
            device.gpioa.pa3,
            device.gpioa.pa6,
            device.gpioa.pa7,
        );
        let motor_aux = (device.tim2, device.tim3);

        BoardConfiguration {
            debug_pin: device.gpioc.pc15,
            spi: device.spi1,
            spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
            ncs: device.gpiob.pb9,
            usart: device.usart2,
            usart_pins: (device.gpioa.pa14, device.gpioa.pa15),
            tx_ch: device.dma_channels.7,
            extih,
            motor_pins,
            motor_aux,
        }
    }

    pub fn setup_motors(
        motor_pins: MotorPins,
        motor_aux: MotorAux,
        clocks: hal::rcc::Clocks,
        freq: Hertz<u32>,
    ) -> Motors {
        // MOTORS:
        // pa0 -- pa3
        let ((ch1, ch2, ch3, ch4), mut timer2) =
            hal::timer::tim2::Timer::new(motor_aux.0, freq, clocks).use_pwm();
        let mut m1_rear_right = pwm!(motor_pins.0, ch1);
        let mut m2_front_right = pwm!(motor_pins.1, ch2);
        let mut m3_rear_left = pwm!(motor_pins.2, ch3);
        let mut m4_front_left = pwm!(motor_pins.3, ch4);
        timer2.enable();

        let max_duty = m1_rear_right.get_max_duty() as f32;

        #[cfg(motors = "motors_quad")]
        {
            #[cfg_attr(rustfmt, rustfmt_skip)]
            let map = [
                [ 1., -1., -1., 1.], /* rear right */
                [1.,  1.,  1., 1.], /* front right */
                [ -1., -1., -1., 1.], /* rear left */
                [-1.,  1.,  1., 1.] /* front left */
            ];
            let pin =
                (m1_rear_right, m2_front_right, m3_rear_left, m4_front_left);
            crate::mixer::Mixer { map, pin, max_duty }
        }

        #[cfg(motors = "motors_hex")]
        {
            let ((ch5, ch6, _, _), mut timer3) =
                hal::timer::tim3::Timer::new(motor_aux.1, freq, clocks)
                    .use_pwm();
            let mut m5_left = pwm!(motor_pins.4, ch5);
            let mut m6_right = pwm!(motor_pins.5, ch6);
            timer3.enable();

            #[cfg_attr(rustfmt, rustfmt_skip)]
            let map = [
                [0.567, -0.815, -1.0, 1.0], /* rear right */
                [0.567, 0.815, -1.0, 1.0], /* front right */
                [-0.567, -0.815, 1.0, 1.0], /* rear left */
                [-0.567, 0.815, 1.0, 1.0], /* front left */
                [-1.0, -0.0, -1.0, 1.0], /* left */
                [1.0, -0.0, 1.0, 1.0] /* right */
            ];

            let pin = (
                m1_rear_right,
                m2_front_right,
                m3_rear_left,
                m4_front_left,
                m5_left,
                m6_right,
            );
            crate::mixer::Mixer { map, pin, max_duty }
        }
    }
}

#[cfg(configuration = "configuration_dev")]
mod defs {
    use super::*;

    pub type DebugPinDef<A, B> = gpio::PA11<A, B>;
    type DT = DebugPinDef<PullNone, Input>;

    pub type NcsPinDef<B> = gpio::PB0<PullNone, B>;
    type NT = NcsPinDef<Input>;
    pub type MpuIntPin = gpio::PA0<PullUp, Input>;

    pub type SpiT = hal::pac::SPI1;
    pub type USART = hal::pac::USART2;
    pub type UsartPins =
        (gpio::PA2<PullNone, Input>, gpio::PA15<PullNone, Input>);
    pub type TxUsart = Tx<USART>;
    pub type RxUsart = Rx<USART>;
    pub type TxCh = hal::dma::dma1::C7;
    pub type ExtiNum = hal::exti::EXTI0;
    pub type MotorPins = ();
    pub type MotorAux = ();

    type Res = BoardConfiguration<
        DT,
        SpiT,
        SpiInputPins,
        NT,
        USART,
        UsartPins,
        TxCh,
        MpuIntPin,
        ExtiNum,
        MotorPins,
        MotorAux,
    >;
    pub fn configure(mut device: Peripherals) -> Res {
        let scl_sck = device.gpiob.pb3;
        let ad0_sdo_miso = device.gpiob.pb4;
        let sda_sdi_mosi = device.gpiob.pb5;

        let mpu_interrupt_pin = device.gpioa.pa0.pull_type(PullUp);
        let mut extih = device
            .exti
            .EXTI0
            .bind(mpu_interrupt_pin, &mut device.syscfg);

        BoardConfiguration {
            debug_pin: device.gpioa.pa11,
            spi: device.spi1,
            spi_pins: (scl_sck, ad0_sdo_miso, sda_sdi_mosi),
            ncs: device.gpiob.pb0,
            usart: device.usart2,
            usart_pins: (device.gpioa.pa2, device.gpioa.pa15),
            tx_ch: device.dma_channels.7,
            extih,
            motor_pins: (),
            motor_aux: (),
        }
    }

    pub fn setup_motors(
        motor_pins: MotorPins,
        motor_aux: MotorAux,
        clocks: hal::rcc::Clocks,
        freq: Hertz<u32>,
    ) -> Motors {
        // no motors in Dev
        ()
    }
}

pub use defs::*;

pub type SCLPin<B> = gpio::PB3<PullNone, B>;
pub type MISOPin<B> = gpio::PB4<PullNone, B>;
pub type MOSIPin<B> = gpio::PB5<PullNone, B>;
pub type SpiInputPins = (SCLPin<Input>, MISOPin<Input>, MOSIPin<Input>);

pub type SpiPins = (
    SCLPin<AltFn<AF5, PushPull, HighSpeed>>,
    MISOPin<AltFn<AF5, PushPull, HighSpeed>>,
    MOSIPin<AltFn<AF5, PushPull, HighSpeed>>,
);

pub type SPI = Spi<SpiT, SpiPins>;
pub type NcsPinT = NcsPinDef<Output<PushPull, HighSpeed>>;
pub type Dev = mpu9250::SpiDevice<SPI, NcsPinT>;
pub type MPU9250 = mpu9250::Mpu9250<Dev, mpu9250::Imu>;

pub type DebugPinT = DebugPinDef<PullNone, Output<PushPull, HighSpeed>>;

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
            let clocks = rcc
                .cfgr
                .sysclk(64.mhz())
                .pclk1(32.mhz())
                .pclk2(32.mhz())
                .freeze(&mut flash.acr);

            Peripherals {
                spi1: device.SPI1,
                spi2: device.SPI2,
                usart1: device.USART1,
                usart2: device.USART2,
                dma_channels,
                exti,
                gpioa,
                gpiob,
                gpioc,
                syscfg,
                clocks,
                tim2: device.TIM2,
                tim3: device.TIM3,
            }
        }
    }

    #[repr(u8)]
    #[derive(Clone, Copy)]
    #[allow(non_camel_case_types)]
    pub enum Interrupt {
        EXTI15_10 = hal::pac::Interrupt::EXTI15_10 as u8,
        EXTI0 = hal::pac::Interrupt::EXTI0 as u8,

        USART2_EXTI26 = hal::pac::Interrupt::USART2_EXTI26 as u8,
    }

    unsafe impl rtfm::export::interrupt::Nr for Interrupt {
        fn nr(&self) -> u8 {
            *self as u8
        }
    }
}
