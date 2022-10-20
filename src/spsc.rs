use heapless::spsc::{Consumer, Producer, Queue};

static mut QUEUE: Queue<u8, 16> = Queue::new();
pub type Tx = Producer<'static, u8, 16>;
pub type Rx = Consumer<'static, u8, 16>;

#[inline]
pub fn pipe() -> (Tx, Rx) {
    unsafe { QUEUE.split() }
}
