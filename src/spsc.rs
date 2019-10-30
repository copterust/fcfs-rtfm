use heapless::consts::*;
use heapless::spsc::{Consumer, Producer, Queue};

static mut QUEUE: Queue<u8, U16> = Queue(heapless::i::Queue::new());
pub type Tx = Producer<'static, u8, U16>;
pub type Rx = Consumer<'static, u8, U16>;

#[inline]
pub fn pipe() -> (Tx, Rx) {
    unsafe { QUEUE.split() }
}
