#![deny(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![allow(unused)]
#![feature(core_intrinsics)]
#![feature(asm)]
#![feature(const_fn)]
#![feature(fn_traits, unboxed_closures)]
#![feature(type_alias_impl_trait)]
#![feature(maybe_uninit_extra)]

use panic_abort;

mod ahrs;
mod boards;
mod chrono;
mod communication;
mod cmd;
mod controllers;
#[macro_use]
mod logging;
mod mixer;
mod prelude;
mod spsc;
mod telemetry;
mod types;
mod utils;

use core::fmt::Write;
use cortex_m_rt::{exception, ExceptionFrame};

use hal::delay::Delay;
use hal::prelude::*;

use asm_delay::{AsmDelay, CyclesToTime};
use cortex_m_log::printer::Printer;
use mpu9250::{Mpu9250, MpuConfig};
use nb::block;
use rtfm::app;

use boards::*;
use prelude::*;
use telemetry::Telemetry;
use mixer::MotorCtrl;

#[app(device = crate::boards::mydevice, peripherals = true)]
const APP: () = {
    struct Resources {
        // ext should be configured in boards
        EXTIH: hal::exti::BoundInterrupt<MpuIntPin, ExtiNum>,
        AHRS: ahrs::AHRS<Dev, chrono::T>,
        LOG: logging::T,
        DEBUG_PIN: DebugPinT,
        // Option is needed to be able to change it in-flight (Option::take)
        CHANNEL: Option<communication::Channel>,
        RX: crate::boards::RxUsart,
        PRODUCER: crate::spsc::Tx,
        CONSUMER: crate::spsc::Rx,
        MOTORS: crate::boards::Motors,
        #[init(crate::types::Control::new())]
        CONTROL: crate::types::Control,
        #[init(crate::types::State::new())]
        STATE: crate::types::State,
    }

    #[init()]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;
        let clocks = device.clocks;
        let mut log = logging::create(ctx.core.ITM).unwrap();
        info!(log, "init!");

        info!(log, "clocks done");
        // This is weird, but gives accurate delays with release
        let mut delay = AsmDelay::new(clocks.sysclk());
        info!(log, "delay ok");

        let mut conf = boards::configure(device);

        let debug_pin = conf.debug_pin
                            .output()
                            .output_speed(HighSpeed)
                            .push_pull()
                            .pull_type(PullNone);

        let mut usart = conf.usart.serial(conf.usart_pins, Bps(460800), clocks);
        usart.listen(hal::serial::Event::Rxne);
        let (tx, rx) = usart.split();

        // SPI1
        let spi = conf.spi.spi(conf.spi_pins, mpu9250::MODE, 1.mhz(), clocks);
        info!(log, "spi ok");

        // MPU
        let ncs_pin = conf.ncs.output().push_pull().output_speed(HighSpeed);
        // 8Hz
        let gyro_rate = mpu9250::GyroTempDataRate::DlpfConf(mpu9250::Dlpf::_2);

        let mut mpu9250 = Mpu9250::imu_with_reinit(
            spi,
            ncs_pin,
            &mut delay,
            &mut MpuConfig::imu()
                .gyro_temp_data_rate(gyro_rate)
                .sample_rate_divisor(3),
            |spi, ncs| {
                let (dev_spi, (scl, miso, mosi)) = spi.free();
                let new_spi = dev_spi.spi((scl, miso, mosi), mpu9250::MODE, 20.mhz(), clocks);
                Some((new_spi, ncs))
            },
        ).unwrap();
        info!(log, "mpu ok");

        mpu9250.enable_interrupts(
            mpu9250::InterruptEnable::RAW_RDY_EN).unwrap();
        info!(log, "int enabled; ");

        info!(log, "now: {:?}", mpu9250.get_enabled_interrupts());
        let mut chrono = chrono::rtfm_stopwatch(clocks.sysclk());
        let mut ahrs = ahrs::AHRS::create(mpu9250, &mut delay, chrono);
        info!(log, "ahrs ok");
        // motors
        let motors = boards::setup_motors(conf.motor_pins,
                                          conf.motor_aux,
                                          clocks,
                                          Hertz(32_000u32));

        info!(log, "ready");
        ahrs.setup_time();

        let (producer, consumer) = spsc::pipe();
        let channel = communication::channel(conf.tx_ch, tx);
        let new_channel = channel.send(|b| utils::fill_with_str(b, "channel ok\r\n"));
        info!(log, "done init");
        init::LateResources { EXTIH: conf.extih,
                              AHRS: ahrs,
                              CHANNEL: Some(new_channel),
                              LOG: log,
                              DEBUG_PIN: debug_pin,
                              RX: rx,
                              PRODUCER: producer,
                              CONSUMER: consumer,
                              MOTORS: motors }
    }

    #[idle(resources=[CONSUMER, CONTROL, CHANNEL])]
    fn idle(mut ctx: idle::Context) -> ! {
        static mut CMD: cmd::Cmd = cmd::create();
        static TELE: telemetry::Telemetry = telemetry::create();
        loop {
            if let Some(byte) = ctx.resources.CONSUMER.dequeue() {
                let (requests, current_control) = ctx.resources.CONTROL.lock(|c| {
                    let requests = CMD.feed(byte, c);
                    (requests, c.clone())
                });
                if requests.status {
                    ctx.resources.CHANNEL.lock(|shared_channel| {
                        let maybe_channel = shared_channel.take();
                        if let Some(channel) = maybe_channel {
                            let new_channel = TELE.control(&current_control,
                                                           channel);
                            *shared_channel = Some(new_channel);
                        }
                    });
                }
            }
        }
    }

    #[task(binds=USART2_EXTI26, resources = [RX, PRODUCER, LOG])]
    fn handle_rx(ctx: handle_rx::Context) {
        let rx = ctx.resources.RX;
        let mut log = ctx.resources.LOG;
        let producer = ctx.resources.PRODUCER;

        match rx.read() {
            Ok(b) => {
                if let Err(e) = producer.enqueue(b) {
                    error!(log, "no space");
                }
            },
            Err(e) => error!(log, "err read"),
        }
    }

    #[task(binds=EXTI15_10,
           resources = [EXTIH, AHRS, LOG, DEBUG_PIN,
                        CHANNEL, CONTROL, STATE, MOTORS])]
    fn handle_mpu_drone(ctx: handle_mpu_drone::Context) {
        #[cfg(configuration = "configuration_drone")]
        handle_mpu(ctx);
    }
    #[task(binds=EXTI0,
           resources = [EXTIH, AHRS, LOG, DEBUG_PIN,
                        CHANNEL, CONTROL, STATE, MOTORS])]
    fn handle_mpu_dev(ctx: handle_mpu_dev::Context) {
        #[cfg(configuration = "configuration_dev")]
        handle_mpu(ctx);
    }
};

#[cfg(configuration = "configuration_drone")]
type CtxType<'a> = handle_mpu_drone::Context<'a>;
#[cfg(configuration = "configuration_dev")]
type CtxType<'a> = handle_mpu_dev::Context<'a>;
fn handle_mpu(mut ctx: CtxType) {
    let _ = ctx.resources.DEBUG_PIN.set_high();
    let mut ahrs = ctx.resources.AHRS;
    let mut log = ctx.resources.LOG;
    let mut state = ctx.resources.STATE;
    let control = ctx.resources.CONTROL;
    static TELE: telemetry::Telemetry = telemetry::create();

    match ahrs.estimate() {
        Ok(result) => {
            state.ahrs = result;
            let (cmd, errors) = controllers::body_rate(&state, &control);
            state.errors = errors;
            state.cmd = cmd;

            ctx.resources.MOTORS.set_duty(cmd[0], cmd[1], cmd[2], control.thrust);

            if control.telemetry {
                let mut maybe_channel = ctx.resources.CHANNEL.take();
                if let Some(channel) = maybe_channel {
                    let new_channel = TELE.state(&state, channel);
                    *ctx.resources.CHANNEL = Some(new_channel);
                }
            }

            debugfloats!(log,
                         ":",
                         result.ypr.yaw,
                         result.ypr.pitch,
                         result.ypr.roll);
        },
        Err(_e) => {
            error!(log, "err");
        },
    };

    let _ = ctx.resources.DEBUG_PIN.set_low();
    ctx.resources.EXTIH.unpend();
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
