use crate::communication::{Channel, TxBuffer};
use crate::types;

pub struct Telemetry;

pub const fn create() -> Telemetry {
    Telemetry
}

// XXX: ufmt
impl Telemetry {
    #[inline]
    pub fn state(&self, state: &types::State, channel: Channel) -> Channel {
        channel.send(|buffer| {
            // tm:ax,ay,az,gx,gy,gz,dt_s,y,p,r,cx,cy,cz
            buffer.push(b't');
            buffer.push(b'm');
            buffer.push(b':');
            for f in state.ahrs.short_results().iter().chain(state.cmd.iter()) {
                let mut b = ryu::Buffer::new();
                let s = b.format(*f);
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(b';');
            }
            buffer.push(b'\n');
        })
    }

    #[inline]
    pub fn control(
        &self,
        control: &types::Control,
        channel: Channel,
    ) -> Channel {
        channel.send(|buffer| {
            // ct:pk,ik,dk,pitch_pk,roll_pk,yaw_pk;
            buffer.push(b'c');
            buffer.push(b't');
            buffer.push(b':');
            for f in control.coefficients().iter() {
                let mut b = ryu::Buffer::new();
                let s = b.format(*f);
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(b';');
            }
            buffer.push(b'\n');
        })
    }
}
