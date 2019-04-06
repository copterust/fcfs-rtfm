use crate::chrono::Chrono;

use dcmimu::DCMIMU;
use ehal::blocking::delay::DelayMs;
use ehal::blocking::spi;
use ehal::digital::OutputPin;
use libm::{asinf, atan2f, fabsf};
use mpu9250::Mpu9250;
use nalgebra::geometry::Quaternion;
use nalgebra::Vector3;

// Magnetometer calibration parameters
// NOTE you need to use the right parameters for *your* magnetometer
// You can use the `log-sensors` example to calibrate your magnetometer. The
// producer is explained in https://github.com/kriswiner/MPU6050/wiki/Simple-and-Effective-Magnetometer-Calibration

// const M_BIAS_X: f32 = 150.;
// const M_SCALE_X: f32 = 0.9;
// const M_BIAS_Y: f32 = -60.;
// const M_SCALE_Y: f32 = 0.9;
// const M_BIAS_Z: f32 = -220.;
// const M_SCALE_Z: f32 = 1.2;

// TODO: make generic over Imu/Marg
pub struct AHRS<DEV, T> {
    mpu: Mpu9250<DEV, mpu9250::Imu>,
    dcmimu: DCMIMU,
    //     accel_biases: Vector3<f32>,
    timer_ms: T,
}

impl<DEV, E, T> AHRS<DEV, T>
    where DEV: mpu9250::Device<Error = E>,
          T: Chrono
{
    pub fn create_calibrated<D>(mut mpu: Mpu9250<DEV, mpu9250::Imu>,
                                delay: &mut D,
                                timer_ms: T)
                                -> Result<Self, mpu9250::Error<E>>
        where D: DelayMs<u8>
    {
        // let mut accel_biases = mpu.calibrate_at_rest(delay)?;
        // Accel biases contain compensation for Earth gravity,
        // so when we will adjust measurements with those biases, gravity will
        // be cancelled. This is helpful for some algos, but not the
        // others. For DCMIMU we need gravity, so we will add it back
        // to measurements, by adjusting biases once.
        // TODO: find real Z axis.
        // accel_biases.z -= mpu9250::G;
        let dcmimu = DCMIMU::new();
        Ok(AHRS { mpu,
                  dcmimu,
                  // accel_biases,
                  timer_ms })
    }

    pub fn setup_time(&mut self) {
        self.timer_ms.reset();
    }

    pub fn estimate(&mut self) -> Result<AhrsResult, E> {
        let meas = self.mpu.all()?;
        let dt_s = self.timer_ms.split_time_s();
        let accel = meas.accel;
        let gyro = meas.gyro;
        let (ypr, gyro_biases) =
            self.dcmimu.update(vec_to_tuple(&gyro), vec_to_tuple(&accel), dt_s);
        let gyro_biases =
            Vector3::new(gyro_biases.x, gyro_biases.y, gyro_biases.z);
        let biased_gyro = gyro - gyro_biases;
        Ok(AhrsResult { ypr, accel, gyro, biased_gyro, dt_s })
    }
}

pub struct AhrsResult {
    pub ypr: dcmimu::EulerAngles,
    pub accel: Vector3<f32>,
    pub gyro: Vector3<f32>,
    pub biased_gyro: Vector3<f32>,
    pub dt_s: f32,
}

fn vec_to_tuple<T: nalgebra::base::Scalar>(inp: &Vector3<T>) -> (T, T, T) {
    (inp.x, inp.y, inp.z)
}
