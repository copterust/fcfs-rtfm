#![deny(warnings)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![allow(unused)]
#![feature(core_intrinsics)]
#![feature(fn_traits, unboxed_closures)]
#![allow(incomplete_features)]
#![feature(type_alias_impl_trait)]

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
use rtic::mutex_prelude::TupleExt02;

use boards::*;
use bootloader::Bootloader;
use mixer::MotorCtrl;
use prelude::*;
use telemetry::Telemetry;

#[app(device = crate::boards::mydevice, peripherals = true)]
mod app {
    use super::*;
    // larhat: try monotonics...
    // #[monotonic(binds = SysTick, default = true)]
    // type DwtMono = DwtSystick<U64, U0, U0>;

    #[local]
    struct Local {
        // ext should be configured in boards
        extih: hal::exti::BoundInterrupt<MpuIntPin, ExtiNum>,
        ahrs: ahrs::AHRS<Dev, chrono::T>,
        cmd: cmd::Cmd,
        debug_pin: DebugPinT,
        rx: crate::boards::RxUsart,
        producer: crate::spsc::Tx,
        consumer: crate::spsc::Rx,
        motors: crate::boards::Motors,
        state: crate::types::State,
        bootloader: crate::bootloader::stm32f30x::Bootloader,
        tele: crate::telemetry::Telemetry,
    }

    #[shared]
    struct Shared {
        log: &'static mut logging::T,
        // Option is needed to be able to change it in-flight (Option::take)
        channel: Option<communication::Channel>,
        control: crate::types::Control,
    }

    #[init()]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
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
        let cmd = cmd::Cmd::new();
        let state = crate::types::State::new();
        let bootloader = crate::bootloader::stm32f30x::Bootloader::new();
        let tele = telemetry::create();

        info!(log, "done init");
        (
            Shared {
                log,
                channel: Some(new_channel),
                control: crate::types::Control::new(),
            },
            Local {
                extih: conf.extih,
                ahrs,
                debug_pin,
                cmd,
                rx,
                producer,
                consumer,
                motors,
                state,
                bootloader,
                tele
            },
            init::Monotonics(),
        )
    }

    #[idle(shared = [channel, control], local = [tele, cmd, consumer, bootloader])]
    fn idle(mut ctx: idle::Context) -> ! {
        let mut tele = ctx.local.tele;
        let mut cmd = ctx.local.cmd;
        let mut consumer = ctx.local.consumer;
        let mut bootloader = ctx.local.bootloader;
        let mut channel = ctx.shared.channel;
        let mut control = ctx.shared.control;
        loop {
            let maybe_byte = consumer.dequeue();

            if let Some(byte) = maybe_byte {
                let (requests, current_control) = control.lock(|c| {
                    let requests = cmd.feed(byte, c);
                    (requests, *c)
                });
                match requests {
                    Some(types::Requests::Status) => {
                        channel.lock(|shared_channel| {
                            let maybe_channel = shared_channel.take();
                            if let Some(channel) = maybe_channel {
                                let new_channel =
                                    tele.control(&current_control, channel);
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

    #[task(binds=USART2_EXTI26, shared = [log], local = [rx, producer])]
    fn handle_rx(mut ctx: handle_rx::Context) {
        let mut rx = ctx.local.rx;
        let mut producer = ctx.local.producer;
        let mut log = ctx.shared.log;

        let input = rx.read();
        match input {
            Ok(b) => {
                if let Err(e) = producer.enqueue(b) {
                    log.lock(|l| error!(l, "no space"));
                }
            }
            Err(e) => log.lock(|l| error!(l, "err read")),
        }
    }

    // #[task(binds=[("configuration_drone", EXTI15_10),
    //               ("configuration_dev", EXTI0)],
    //        local = [extih, ahrs, log, debug_pin,
    //                 channel, control, state, motors])]
    #[task(binds=EXTI0,
           shared = [log, channel, control],
           local = [extih, ahrs, debug_pin, state, motors])]
    fn handle_mpu(mut ctx: handle_mpu::Context) {
        static TELE: telemetry::Telemetry = telemetry::create();
        // shared
        let control = ctx.shared.control.lock(|c| c.clone());

        let estimation = ctx.local.ahrs.estimate();
        match estimation {
            Ok(result) => {
                let (cmd, errors) = controllers::body_rate(&ctx.local.state, &control);
                // update state
                *ctx.local.state = types::State {
                    ahrs: result,
                    cmd,
                    errors
                };

                ctx.local.motors.set_duty(cmd[0], cmd[1], cmd[2], control.thrust);
                let curren_state = ctx.local.state.clone();
                if control.telemetry {
                    ctx.shared.channel.lock(|maybe_channel| {
                        if let Some(in_channel) = maybe_channel.take() {
                            let new_channel = TELE.state(&curren_state, in_channel);
                            *maybe_channel = Some(new_channel);
                        }
                    });
                }

                ctx.shared.log.lock(|l| {
                    debugfloats!(
                        l,
                        ":",
                        result.ypr.yaw,
                        result.ypr.pitch,
                        result.ypr.roll
                    );
                });
            }
            Err(_e) => {
                ctx.shared.log.lock(|l| error!(l, "err"));
            }
        };

        ctx.local.debug_pin.set_low();
        ctx.local.extih.unpend();
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
