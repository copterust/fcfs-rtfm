use crate::communication::{Channel, TxBuffer};
use crate::ahrs::AhrsResult;
use crate::ahrs::AhrsShortResult;


pub struct Telemetry;

pub fn create() -> Telemetry {
    return Telemetry
}

impl Telemetry {
    #[inline]
    pub fn report(&self, arg: &AhrsResult, channel: Channel) -> Channel {
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
