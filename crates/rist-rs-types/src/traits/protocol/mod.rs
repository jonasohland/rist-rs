use std::fmt::Debug;

use super::{
    runtime::{self, Runtime},
    time::clock::Clock,
};

pub trait Ctl: Debug + Clone + Sized + Send + 'static {
    type Error: Debug;
    type Output;

    fn start() -> Self;
    fn shutdown() -> Self;
}

pub struct ProtocolEvent<R>
where
    R: Runtime,
{
    _next_wake: <R::Clock as Clock>::TimePoint,
}

impl<R> ProtocolEvent<R>
where
    R: Runtime,
{
    pub fn asap(clock: &R::Clock) -> Self {
        Self {
            _next_wake: clock.immediate(),
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct ReadyFlags: u32 {
        const Writable = 0x01;
        const Readable = 0x02;
    }
}

pub enum IOV<R: Runtime, C: Ctl> {
    Ready(R::Socket, ReadyFlags),
    Error(R::Socket, runtime::Error),
    Ctl(C),
    Empty,
}

impl<R: Runtime, C: Ctl> Clone for IOV<R, C> {
    fn clone(&self) -> Self {
        match self {
            Self::Ready(arg0, arg1) => Self::Ready(*arg0, *arg1),
            Self::Error(arg0, arg1) => Self::Error(*arg0, arg1.clone()),
            Self::Ctl(arg0) => Self::Ctl(arg0.clone()),
            Self::Empty => Self::Empty,
        }
    }
}

impl<R: Runtime, C: Ctl> Debug for IOV<R, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(arg0, arg1) => f.debug_tuple("Readable").field(arg0).field(arg1).finish(),
            Self::Error(arg0, arg1) => f.debug_tuple("Error").field(arg0).field(arg1).finish(),
            Self::Ctl(arg0) => f.debug_tuple("Ctl").field(arg0).finish(),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl<R: Runtime, C: Ctl> Default for IOV<R, C> {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug)]
pub struct Events<R: Runtime, C: Ctl>(Vec<IOV<R, C>>);

impl<R: Runtime, C: Ctl> Events<R, C> {
    pub fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn push(&mut self, ev: IOV<R, C>) {
        self.0.push(ev)
    }

    pub fn pop(&mut self) -> Option<IOV<R, C>> {
        self.0.pop()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub trait Protocol<R>: Sized + Send + 'static
where
    R: Runtime,
{
    type Ctl: Ctl;

    fn run(&mut self, rt: &mut R, iov: &[IOV<R, Self::Ctl>]) -> ProtocolEvent<R>;
}
