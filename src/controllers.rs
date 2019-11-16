use crate::ahrs::AhrsResult;
use crate::prelude::*;
use crate::types;
use crate::utils::to_rads;

// "body rate" controller from f3-eva
// XXX: return types?
// returns (corrections, errors)
pub fn body_rate(
    state: &types::State,
    control: &types::Control,
) -> ([f32; 3], [f32; 3]) {
    let pitch_target = to_rads(control.target_degrees.pitch);
    let roll_target = to_rads(control.target_degrees.roll);
    let yaw_target = to_rads(control.target_degrees.yaw);

    let pitch_err = (pitch_target - state.ahrs.ypr.pitch) * control.pitch_pk;
    let yaw_err = (yaw_target - state.ahrs.ypr.yaw) * control.yaw_pk;
    let roll_err = (roll_target - state.ahrs.ypr.roll) * control.roll_pk;

    // XXX?
    let x_err = roll_err - state.ahrs.biased_gyro[0];
    let y_err = pitch_err - state.ahrs.biased_gyro[1];
    let z_err = yaw_err - state.ahrs.biased_gyro[2];
    // ?XXX

    let i_comp = 0.;
    let delta_x = x_err - state.errors[0];
    let delta_y = y_err - state.errors[1];
    let delta_z = z_err - state.errors[2];
    let x_corr =
        x_err * control.pk + i_comp * control.ik + control.dk * delta_x;
    let y_corr =
        y_err * control.pk + i_comp * control.ik + control.dk * delta_y;
    let z_corr = 0.; // z_err * control.pk + i_comp * control.ik + control.dk * delta_z;

    ([x_corr, y_corr, z_corr], [x_err, y_err, z_err])
}
