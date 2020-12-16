use crate::boards::*;

use heapless::consts::*;
use heapless::Vec;

pub type TxBuffer = Vec<u8, U256>;
type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
type TxBusy = dma::Transfer<dma::R, &'static mut TxBuffer, TxCh, TxUsart>;

static mut BUFFER: TxBuffer = Vec(heapless::i::Vec::new());

pub fn channel(ch: crate::boards::TxCh, tx: crate::boards::TxUsart) -> Channel {
    Channel::create(ch, tx)
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
    where
        F: for<'a> FnMut<(&'a mut TxBuffer,), Output = ()>,
    {
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
