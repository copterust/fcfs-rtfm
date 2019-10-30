use crate::communication::{Channel, TxBuffer};
use crate::ahrs::AhrsResult;
use crate::ahrs::AhrsShortResult;


pub trait Telemetry {
    fn report(&self, arg: &AhrsResult, channel: Channel) -> Channel;
}

pub type T = impl Telemetry;

pub struct Dummy;
pub struct Bytes;
pub struct Words;

#[allow(unused)]
pub fn create() -> T {
    #[cfg(telemetry = "telemetry_dummy")]
    return Dummy;
    #[cfg(telemetry = "telemetry_words")]
    return Words;
}


impl Telemetry for Dummy {
    #[inline]
    fn report(&self, arg: &AhrsResult, channel: Channel) -> Channel {
        channel
    }
}


impl Telemetry for Words {
    #[inline]
    fn report(&self, arg: &AhrsResult, channel: Channel) -> Channel {
        channel.send(|buffer| {
            // tm:ax,ay,az,gx,gy,gz,dt_s,y,p,r
            buffer.push('t' as u8);
            buffer.push('m' as u8);
            buffer.push(':' as u8);
            for f in arg.short_results().into_iter() {
                let mut b = ryu::Buffer::new();
                let s = b.format(*f);
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(';' as u8);
            }
            buffer.push('\n' as u8);
        })
    }
}
