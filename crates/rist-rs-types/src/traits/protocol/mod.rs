use std::fmt::Debug;

use super::{runtime::Runtime, time::clock::Clock};

pub trait Ctl: Sized + Send + 'static {
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

    fn ctl(
        &mut self,
        rt: &mut R,
        op: Self::Ctl,
    ) -> Result<<Self::Ctl as Ctl>::Output, <Self::Ctl as Ctl>::Error>;

    /// Accept a new connection from a remote entity
    fn accept(
        &mut self,
        rt: &mut R,
        local_socket: R::Socket,
        remote_socket: R::Socket,
        remote_address: R::SocketAddr,
    ) -> ProtocolEvent<R>;

    fn receive(&mut self, rt: &mut R, socket: R::Socket, buf: &[u8]) -> ProtocolEvent<R>;

    fn writeable(&mut self, rt: &mut R, socket: R::Socket) -> ProtocolEvent<R>;

    fn wake(&mut self, rt: &mut R) -> ProtocolEvent<R>;
}
