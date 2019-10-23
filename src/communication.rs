use crate::boards::*;

use heapless::consts::*;
use heapless::Vec;

pub type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy = dma::Transfer<dma::R, &'static mut TxBuffer, TxCh, TxUsart>;

static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());

pub fn create_channel(ch: crate::boards::TxCh, tx: crate::boards::TxUsart) -> Channel {
    return Channel::create(ch, tx);
}

enum TransferState {
    Ready(TxReady),
    MaybeBusy(TxBusy),
}

pub struct Channel {
    state: TransferState,
}

impl Channel {
    fn with_state(ns: TransferState) -> Self {
        Channel { state: ns }
    }

    fn create(ch: TxCh, tx: TxUsart) -> Self {
        let bf = unsafe { &mut BUFFER };
        let state = TransferState::Ready((bf, ch, tx));
        Channel::with_state(state)
    }

    pub fn send<F>(self, mut buffer_filler: F) -> Self
    where F: for<'a> FnMut<(&'a mut TxBuffer,), Output = ()> {
        let ns = match self.state {
            TransferState::Ready((mut buffer, ch, tx)) => {
                buffer_filler(&mut buffer);
                TransferState::MaybeBusy(tx.write_all(ch, buffer))
            }
            TransferState::MaybeBusy(transfer) => {
                if transfer.is_done() {
                    let (buffer, ch, tx) = transfer.wait();
                    buffer.clear();
                    TransferState::Ready((buffer, ch, tx))
                } else {
                    // not ready yet, skip tansfer
                    // XXX: alternatevely, we can allocate bigger buffer
                    //      and use its chunks.
                    TransferState::MaybeBusy(transfer)
                }
            }
        };

        match ns {
            TransferState::MaybeBusy(_) => Channel::with_state(ns),
            TransferState::Ready(_) => {
                Channel::with_state(ns).send(buffer_filler)
            }
        }
    }
}

// const MAGIC: [u8; 3] = [108, 111, 108];
// pub struct ByteWriter;
// impl ByteWriter {
//     fn store_float_as_bytes(buffer: &mut TxBuffer, f: f32) {
//         let bits = f.to_bits();
//         let bytes: [u8; 4] = unsafe { core::mem::transmute(bits) };
//         for byte in bytes.iter() {
//             buffer.push(*byte);
//         }
//     }
// }
// impl TelemetryWriter for ByteWriter {
//     type Arg = super::AhrsResult;

//     fn write_arg(buffer: &mut TxBuffer, arg: &Self::Arg) {
//         buffer.extend_from_slice(&MAGIC);
//         // ax,ay,az,gx,gy,gz,dt_s,y,p,r
//         for f in arg.short_results().into_iter() {
//             ByteWriter::store_float_as_bytes(buffer, *f);
//         }
//     }
// }

// pub struct WordWriter;
// impl TelemetryWriter for WordWriter {
//     type Arg = super::AhrsResult;

//     fn write_arg(buffer: &mut TxBuffer, arg: &Self::Arg) {
//         // ax,ay,az,gx,gy,gz,dt_s,y,p,r
//         for f in arg.short_results().into_iter() {
//             let mut b = ryu::Buffer::new();
//             let s = b.format(*f);
//             buffer.extend_from_slice(s.as_bytes());
//             buffer.push(';' as u8);
//         }
//         buffer.push('\n' as u8);
//     }
// }
