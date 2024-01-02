use core::fmt::{Debug, Display};
use core::hash::Hash;

use super::time::clock::Clock;

mod error;
mod event;
mod events;

pub use error::Error;
pub use event::Event;
pub use event::Readiness;
pub use events::Events;

use rist_rs_macros::{cfg_no_std, cfg_std};

cfg_std! {
    pub trait SocketAddr: Debug + Display + Clone + Copy + PartialEq + Eq + Hash + Send
    where
        Self: From<std::net::SocketAddr>,
    {
    }
}

cfg_no_std! {
    pub trait SocketAddr: Debug + Display + Clone + Copy + PartialEq + Eq + Hash + Send
    {
    }
}

pub trait Socket: Debug + Display + Clone + Copy + PartialEq + Eq + Hash + Send {}

pub trait Runtime: Send + 'static {
    type Clock: Clock;

    type SocketAddr: SocketAddr;

    type Socket: Socket;

    fn get_clock(&mut self, id: Option<&str>) -> Self::Clock;

    fn get_default_clock(&mut self) -> Self::Clock {
        self.get_clock(None)
    }

    fn get_remote_address(&self, remote: Self::Socket) -> Result<Self::SocketAddr, Error>;

    fn bind(&mut self, address: Self::SocketAddr) -> Result<Self::Socket, Error>;

    fn connect(&mut self, socket: Self::Socket, address: Self::SocketAddr) -> Result<(), Error>;

    fn send(&mut self, socket: Self::Socket, buf: &[u8]) -> Result<(), Error>;

    fn send_to(
        &mut self,
        socket: Self::Socket,
        buf: &[u8],
        address: Self::SocketAddr,
    ) -> Result<(), Error>;

    fn recv(&mut self, socket: Self::Socket, buf: &mut [u8]) -> Result<usize, Error>;

    fn recv_from(
        &mut self,
        socket: Self::Socket,
        buf: &mut [u8],
    ) -> Result<(usize, Self::SocketAddr), Error>;

    fn close(&mut self, socket: Self::Socket);
}
