use core::{
    ptr,
    task::{RawWaker, RawWakerVTable, Waker},
};

unsafe fn noop(_p: *const ()) {}

unsafe fn noop_clone(_p: *const ()) -> RawWaker {
    RawWaker::new(ptr::null(), &VTABLE)
}

const VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

pub fn noop_waker() -> Waker {
    let raw = RawWaker::new(ptr::null(), &VTABLE);
    unsafe { Waker::from_raw(raw) }
}
