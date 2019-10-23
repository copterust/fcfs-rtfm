use crate::communication::TxBuffer;
use crate::ahrs::AhrsResult;
use crate::ahrs::AhrsShortResult;


pub trait Telemetry {
    fn report(&self, arg: &AhrsResult, destination: &mut TxBuffer);
}

pub type T = impl Telemetry;

pub struct Dummy;
pub struct Bytes;
pub struct Words;

#[allow(unused)]
pub fn create() -> T {
    #[cfg(telemetry = "telemetry_dummy")]
    return Dummy;
    #[cfg(telemetry = "telemetry_bytes")]
    return Bytes;
    #[cfg(telemetry = "telemetry_words")]
    return Words;
}


impl Telemetry for Dummy {
    #[inline]
    fn report(&self, arg: &AhrsResult, destination: &mut TxBuffer) {
    }
}

const MAGIC: [u8; 3] = [108, 111, 108];
impl Telemetry for Bytes {
    fn report(&self, arg: &AhrsResult, buffer: &mut TxBuffer) {
        buffer.extend_from_slice(&MAGIC);
        // ax,ay,az,gx,gy,gz,dt_s,y,p,r
        for f in arg.short_results().into_iter() {
            store_float_as_bytes(buffer, *f);
        }
    }
}
fn store_float_as_bytes(buffer: &mut TxBuffer, f: f32) {
    let bits = f.to_bits();
    let bytes: [u8; 4] = unsafe { core::mem::transmute(bits) };
    for byte in bytes.iter() {
        buffer.push(*byte);
    }
}

impl Telemetry for Words {
    fn report(&self, arg: &AhrsResult, buffer: &mut TxBuffer) {
        // ax,ay,az,gx,gy,gz,dt_s,y,p,r
        for f in arg.short_results().into_iter() {
            let mut b = ryu::Buffer::new();
            let s = b.format(*f);
            buffer.extend_from_slice(s.as_bytes());
            buffer.push(';' as u8);
        }
        buffer.push('\n' as u8);
    }
}
