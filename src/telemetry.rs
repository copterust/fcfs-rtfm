use crate::communication::{Channel, TxBuffer};
use crate::types;


pub struct Telemetry;

pub const fn create() -> Telemetry {
    return Telemetry
}

// XXX: ufmt
impl Telemetry {
    #[inline]
    pub fn state(&self, state: &types::State, channel: Channel) -> Channel {
        channel.send(|buffer| {
            // tm:ax,ay,az,gx,gy,gz,dt_s,y,p,r,cx,cy,cz
            buffer.push('t' as u8);
            buffer.push('m' as u8);
            buffer.push(':' as u8);
            for f in state.ahrs.short_results().into_iter().chain(state.cmd.into_iter()) {
                let mut b = ryu::Buffer::new();
                let s = b.format(*f);
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(';' as u8);
            }
            buffer.push('\n' as u8);
        })
    }

    #[inline]
    pub fn coefficients(&self, coefs: &[f32], channel: Channel) -> Channel {
        channel.send(|buffer| {
            // ct:pk,ik,dk,pitch_pk,roll_pk,yaw_pk;
            buffer.push('c' as u8);
            buffer.push('t' as u8);
            buffer.push(':' as u8);
            for f in coefs.into_iter() {
                let mut b = ryu::Buffer::new();
                let s = b.format(*f);
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(';' as u8);
            }
            buffer.push('\n' as u8);
        })
    }
}
