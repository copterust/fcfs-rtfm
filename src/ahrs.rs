use crate::chrono::Chrono;

use dcmimu::{DCMIMU, EulerAngles};
use ehal::blocking::delay::DelayMs;
use ehal::blocking::spi;
use ehal::digital::OutputPin;
use libm::{asinf, atan2f, atanf, fabsf, sqrtf};
use mpu9250::{Mpu9250, Vector3};

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
//

mod kalman;

// TODO: make generic over Imu/Marg
pub struct AHRS<DEV, T> {
    mpu: Mpu9250<DEV, mpu9250::Imu>,
    dcmimu: DCMIMU,
    //     accel_biases: Vector3<f32>,
    timer_ms: T,
    angle_x: f32,
    angle_y: f32,
    kalman_x: kalman::AngularKalman,
    kalman_y: kalman::AngularKalman,
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
        let ky = kalman::AngularKalman{
            q_a: 0.001,
            q_b: 0.002,
            r: 0.04,
            angle: 0.0,
            bias: 0.0,
            rate: 0.0,
            p: [[0.0, 0.0], [0.0, 0.0]],
            k: [0.0, 0.0],
            y: 0.0,
            s: 0.0
        };

        let kx = kalman::AngularKalman{
            q_a: 0.001,
            q_b: 0.002,
            r: 0.04,
            angle: 0.0,
            bias: 0.0,
            rate: 0.0,
            p: [[0.0, 0.0], [0.0, 0.0]],
            k: [0.0, 0.0],
            y: 0.0,
            s: 0.0
        };

        let dcmimu = DCMIMU::new();
        Ok(AHRS { mpu,
                  dcmimu,
                  // accel_biases,
                  timer_ms,
                  angle_x: 0.0,
                  angle_y: 0.0,
                  kalman_x: kx,
                  kalman_y: ky,
                  })
    }

    pub fn setup_time(&mut self) {
        self.timer_ms.reset();
    }

    pub fn estimate(&mut self) -> Result<AhrsResult, E> {
        let meas = self.mpu.all()?;
        let dt_s = self.timer_ms.split_time_s();
        let accel = meas.accel;
        let gyro = meas.gyro;

        // New filter
        let roll  = atan2f(accel[1], accel[2]);
        let pitch = atanf(-accel[0] / sqrtf(accel[1] * accel[1] + accel[2] * accel[2]));

        let gyro_x = gyro[0];
        let mut gyro_y = gyro[1];

        if ((roll < -1.57 && self.angle_x > 1.57) || (roll > 1.57 && self.angle_x < -1.57)) {
            self.kalman_x.set_angle(roll);
            self.angle_x = roll;
        } else {
            self.angle_x = self.kalman_x.step(roll, gyro_x, dt_s);
        }
        if (fabsf(self.angle_x) > 1.57) {
            gyro_y = -gyro_y;
        }
        self.angle_y = self.kalman_y.step(pitch, gyro_y, dt_s);
        // retlif weN

        let (ypr, gyro_biases) =
            self.dcmimu.update(vec_to_tuple(&gyro), vec_to_tuple(&accel), dt_s);
        let gyro_biases =
            Vector3::new(gyro_biases.x, gyro_biases.y, gyro_biases.z);
        let biased_gyro = gyro - gyro_biases;
        let klmypr = EulerAngles {
            yaw: ypr.yaw,
            pitch: self.angle_x,
            roll: self.angle_y,
        };
        Ok(AhrsResult { ypr: klmypr, accel, gyro, biased_gyro, dt_s })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AhrsResult {
    pub accel: Vector3<f32>,
    pub gyro: Vector3<f32>,
    pub dt_s: f32,
    pub ypr: dcmimu::EulerAngles,
    pub biased_gyro: Vector3<f32>,
}

impl AhrsResult {
    /// ax,ay,az,gx,gy,gz,dt_s,y,p,r
    pub fn short_results(&self) -> [f32; 10] {
        [self.accel.x,
         self.accel.y,
         self.accel.z,
         self.gyro.x,
         self.gyro.y,
         self.gyro.z,
         self.dt_s,
         self.ypr.yaw,
         self.ypr.pitch,
         self.ypr.roll]
    }

    // ax,ay,az,gx,gy,gz,dt_s,y,p,r,bgx,bgy,bgz
    pub fn full_results(&self) -> [f32; 13] {
        [self.accel.x,
         self.accel.y,
         self.accel.z,
         self.gyro.x,
         self.gyro.y,
         self.gyro.z,
         self.dt_s,
         self.ypr.yaw,
         self.ypr.pitch,
         self.ypr.roll,
         self.biased_gyro.x,
         self.biased_gyro.y,
         self.biased_gyro.z]
    }
}

fn vec_to_tuple(inp: &Vector3<f32>) -> (f32, f32, f32) {
    (inp.x, inp.y, inp.z)
}
