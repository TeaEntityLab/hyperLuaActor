use futures::{Async, Future, Poll};
use hyper::Error;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct HyperLatch {
    is_alive: AtomicBool,
}
impl Default for HyperLatch {
    fn default() -> Self {
        HyperLatch {
            is_alive: AtomicBool::new(true),
        }
    }
}
impl Future for HyperLatch {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.is_alive.load(Ordering::SeqCst) {
            Ok(Async::NotReady)
        } else {
            Ok(Async::Ready(()))
        }
    }
}
