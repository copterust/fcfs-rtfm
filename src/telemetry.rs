use crate::ahrs::AhrsResult;

pub trait Telemetry {
    type Arg;

    fn send(self, arg: &Self::Arg) -> Self;
}

pub existential type T: Telemetry<Arg = AhrsResult>;

#[allow(unused)]
pub fn create(ch: crate::boards::TxCh, tx: crate::boards::TxUsart) -> T {
    #[cfg(telemetry = "telemetry_dummy")]
    return dummy::make();
    #[cfg(telemetry = "telemetry_bytes")]
    return dmatelemetry::DmaTelemetry::create(ch,
                                              tx,
                                              dmatelemetry::ByteWriter);
    #[cfg(telemetry = "telemetry_words")]
    return dmatelemetry::DmaTelemetry::create(ch,
                                              tx,
                                              dmatelemetry::WordWriter);
}

mod dummy {
    pub struct Dummy;

    impl super::Telemetry for Dummy {
        type Arg = super::AhrsResult;

        fn send(self, arg: &Self::Arg) -> Self {
            self
        }
    }

    pub fn make() -> Dummy {
        Dummy
    }
}

#[cfg(any(telemetry = "telemetry_bytes", telemetry = "telemetry_words"))]
mod dmatelemetry {
    use crate::boards::*;

    use heapless::consts::*;
    use heapless::Vec;

    type TxBuffer = Vec<u8, U256>;
    type TxReady = (&'static mut TxBuffer, TxCh, TxUsart);
    type TxBusy = dma::Transfer<dma::R, &'static mut TxBuffer, TxCh, TxUsart>;

    static mut BUFFER: TxBuffer = Vec::new();

    #[derive(Debug, Clone, Copy)]
    enum TransferState<Ready, Busy> {
        Ready(Ready),
        MaybeBusy(Busy),
    }

    pub trait TelemetryWriter {
        type Arg;
        fn write_arg(buffer: &mut TxBuffer, arg: &Self::Arg);
    }

    pub struct DmaTelemetry<Ready, Busy, TW> {
        state: TransferState<Ready, Busy>,
        _tw: core::marker::PhantomData<TW>,
    }

    impl<Ready, Busy, TW> DmaTelemetry<Ready, Busy, TW> {
        fn with_state(ns: TransferState<Ready, Busy>) -> Self {
            DmaTelemetry { state: ns, _tw: core::marker::PhantomData }
        }
    }

    impl<TW: TelemetryWriter<Arg = super::AhrsResult>>
        DmaTelemetry<TxReady, TxBusy, TW>
    {
        pub fn create(ch: TxCh, tx: TxUsart, _tw: TW) -> Self {
            let bf = unsafe { &mut BUFFER };
            let state = TransferState::Ready((bf, ch, tx));
            DmaTelemetry::with_state(state)
        }
    }

    impl<TW: TelemetryWriter<Arg = super::AhrsResult>> super::Telemetry
        for DmaTelemetry<TxReady, TxBusy, TW>
    {
        type Arg = super::AhrsResult;

        fn send(self, arg: &Self::Arg) -> Self {
            let ns = match self.state {
                TransferState::Ready((mut buffer, ch, tx)) => {
                    TW::write_arg(&mut buffer, &arg);
                    TransferState::MaybeBusy(tx.write_all(ch, buffer))
                },
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
                },
            };

            DmaTelemetry::with_state(ns)
        }
    }

    pub struct ByteWriter;
    impl ByteWriter {
        fn store_float_as_bytes(buffer: &mut TxBuffer, f: f32) {
            let bits = f.to_bits();
            let bytes: [u8; 4] = unsafe { core::mem::transmute(bits) };
            for byte in bytes.iter() {
                buffer.push(*byte);
            }
        }
    }
    impl TelemetryWriter for ByteWriter {
        type Arg = super::AhrsResult;

        fn write_arg(buffer: &mut TxBuffer, arg: &Self::Arg) {
            buffer.push(0);
            // ax,ay,az,gx,gy,gz,dt_s,y,p,r
            for f in arg.short_results().into_iter() {
                ByteWriter::store_float_as_bytes(buffer, *f);
            }
            buffer.push(0);
        }
    }

    pub struct WordWriter;
    impl TelemetryWriter for WordWriter {
        type Arg = super::AhrsResult;

        fn write_arg(buffer: &mut TxBuffer, arg: &Self::Arg) {
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

}
