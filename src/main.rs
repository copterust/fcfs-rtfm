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
#![feature(llvm_asm)]
#![feature(const_impl_trait)]

mod ahrs;
#[macro_use]
mod logging;
mod blackbox;
mod boards;
mod bootloader;
mod chrono;
mod cmd;
mod communication;
mod controllers;
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
use rtic::app;

use boards::*;
use bootloader::Bootloader;
use mixer::MotorCtrl;
use prelude::*;
use telemetry::Telemetry;

#[app(device = crate::boards::mydevice, peripherals = true)]
const APP: () = {
    struct Resources {
        // ext should be configured in boards
        extih: hal::exti::BoundInterrupt<MpuIntPin, ExtiNum>,
        ahrs: ahrs::AHRS<Dev, chrono::T>,
        log: &'static mut logging::T,
        debug_pin: DebugPinT,
        // Option is needed to be able to change it in-flight (Option::take)
        channel: Option<communication::Channel>,
        rx: crate::boards::RxUsart,
        producer: crate::spsc::Tx,
        consumer: crate::spsc::Rx,
        motors: crate::boards::Motors,
        #[init(crate::types::Control::new())]
        control: crate::types::Control,
        #[init(crate::types::State::new())]
        state: crate::types::State,
        #[init(crate::bootloader::create())]
        bootloader: crate::bootloader::T,
    }

    #[init()]
    fn init(ctx: init::Context) -> init::LateResources {
        let device = ctx.device;
        let clocks = device.clocks;
        let raw_log = logging::create(ctx.core.ITM).unwrap();
        let log = blackbox::init(raw_log);
        info!(log, "init!");

        info!(log, "clocks done");
        // This is weird, but gives accurate delays with release
        let mut delay = AsmDelay::new(clocks.sysclk());
        info!(log, "delay ok");

        let mut conf = boards::configure(device);

        let debug_pin = conf
            .debug_pin
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
                let new_spi = dev_spi.spi(
                    (scl, miso, mosi),
                    mpu9250::MODE,
                    20.mhz(),
                    clocks,
                );
                Some((new_spi, ncs))
            },
        )
        .unwrap();
        info!(log, "mpu ok");

        mpu9250
            .enable_interrupts(mpu9250::InterruptEnable::RAW_RDY_EN)
            .unwrap();
        info!(log, "int enabled; ");

        info!(log, "now: {:?}", mpu9250.get_enabled_interrupts());
        let mut chrono = chrono::rtfm_stopwatch(clocks.sysclk());
        let mut ahrs = ahrs::AHRS::create(mpu9250, &mut delay, chrono);
        info!(log, "ahrs ok");
        // motors
        let motors = boards::setup_motors(
            conf.motor_pins,
            conf.motor_aux,
            clocks,
            Hertz(32_000u32),
        );

        info!(log, "ready");
        ahrs.setup_time();

        let (producer, consumer) = spsc::pipe();
        let channel = communication::channel(conf.tx_ch, tx);
        let new_channel =
            channel.send(|b| utils::fill_with_str(b, "channel ok\r\n"));
        info!(log, "done init");
        init::LateResources {
            extih: conf.extih,
            ahrs,
            channel: Some(new_channel),
            log,
            debug_pin,
            rx,
            producer,
            consumer,
            motors,
        }
    }

    #[idle(resources=[consumer, control, channel, bootloader])]
    fn idle(mut ctx: idle::Context) -> ! {
        static mut CMD: cmd::Cmd = cmd::create();
        static TELE: telemetry::Telemetry = telemetry::create();
        let idle::Resources {
            mut consumer,
            mut channel,
            mut control,
            mut bootloader,
        } = ctx.resources;
        loop {
            if let Some(byte) = consumer.dequeue() {
                let (requests, current_control) = control.lock(|c| {
                    let requests = CMD.feed(byte, c);
                    (requests, c.clone())
                });
                match requests {
                    Some(types::Requests::Status) => {
                        channel.lock(|shared_channel| {
                            let maybe_channel = shared_channel.take();
                            if let Some(channel) = maybe_channel {
                                let new_channel =
                                    TELE.control(&current_control, channel);
                                *shared_channel = Some(new_channel);
                            }
                        });
                    }
                    Some(types::Requests::Boot) => {
                        bootloader.to_bootloader();
                    }
                    Some(types::Requests::Reset) => {
                        bootloader.system_reset();
                    }
                    _ => {}
                }
            }
        }
    }

    #[task(binds=USART2_EXTI26, resources = [rx, producer, log])]
    fn handle_rx(ctx: handle_rx::Context) {
        let handle_rx::Resources {
            rx,
            mut log,
            producer,
        } = ctx.resources;

        match rx.read() {
            Ok(b) => {
                if let Err(e) = producer.enqueue(b) {
                    error!(log, "no space");
                }
            }
            Err(e) => error!(log, "err read"),
        }
    }

    #[task(binds=EXTI15_10, resources = [extih, ahrs, log, debug_pin,
                                         channel, control, state, motors])]
    fn handle_mpu_drone(ctx: handle_mpu_drone::Context) {
        #[cfg(configuration = "configuration_drone")]
        handle_mpu(ctx);
    }
    #[task(binds=EXTI0, resources = [extih, ahrs, log, debug_pin,
                                     channel, control, state, motors])]
    fn handle_mpu_dev(ctx: handle_mpu_dev::Context) {
        #[cfg(configuration = "configuration_dev")]
        handle_mpu(ctx);
    }
};

#[cfg(configuration = "configuration_drone")]
type CtxType<'a> = handle_mpu_drone::Context<'a>;
#[cfg(configuration = "configuration_drone")]
type ResourceType<'a> = handle_mpu_drone::Resources<'a>;
#[cfg(configuration = "configuration_dev")]
type CtxType<'a> = handle_mpu_dev::Context<'a>;
#[cfg(configuration = "configuration_dev")]
type ResourceType<'a> = handle_mpu_dev::Resources<'a>;
fn handle_mpu(mut ctx: CtxType) {
    static TELE: telemetry::Telemetry = telemetry::create();
    let ResourceType {
        mut debug_pin,
        mut ahrs,
        mut state,
        mut log,
        mut motors,
        mut channel,
        mut extih,
        control,
    } = ctx.resources;

    match ahrs.estimate() {
        Ok(result) => {
            state.ahrs = result;
            let (cmd, errors) = controllers::body_rate(&state, &control);
            state.errors = errors;
            state.cmd = cmd;

            motors.set_duty(cmd[0], cmd[1], cmd[2], control.thrust);

            if control.telemetry {
                let mut maybe_channel = channel.take();
                if let Some(in_channel) = maybe_channel {
                    let new_channel = TELE.state(&state, in_channel);
                    *channel = Some(new_channel);
                }
            }

            debugfloats!(
                log,
                ":",
                result.ypr.yaw,
                result.ypr.pitch,
                result.ypr.roll
            );
        }
        Err(_e) => {
            error!(log, "err");
        }
    };

    debug_pin.set_low();
    extih.unpend();
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
