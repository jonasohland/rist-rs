use std::fmt::Debug;

use super::{
    runtime::{Event, Runtime},
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

pub trait Protocol<R>: Sized + Send + 'static
where
    R: Runtime,
{
    type Ctl: Ctl;

    fn run(&mut self, rt: &mut R, iov: &[Event<R, Self::Ctl>]) -> ProtocolEvent<R>;
}
