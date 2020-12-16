use crate::communication::TxBuffer;
use core::f32::consts::PI;

pub fn fill_with_bytes(buffer: &mut TxBuffer, arg: &[u8]) {
    buffer.extend_from_slice(arg).unwrap();
}

pub fn fill_with_str(buffer: &mut TxBuffer, arg: &str) {
    buffer.extend_from_slice(arg.as_bytes()).unwrap();
}

pub fn to_rads(d: f32) -> f32 {
    d * PI / 180.
}
