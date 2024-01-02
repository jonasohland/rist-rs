use core::{future::Future, pin::pin};

pub mod noop_waker;

use self::noop_waker::noop_waker;

pub fn try_poll_once<T>(f: impl Future<Output = T>) -> Option<T> {
    let waker = noop_waker();
    let mut cx = core::task::Context::from_waker(&waker);
    match pin!(f).poll(&mut cx) {
        core::task::Poll::Ready(t) => Some(t),
        core::task::Poll::Pending => None,
    }
}
