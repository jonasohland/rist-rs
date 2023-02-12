use std::{
    fmt::{Debug, Display},
    hash::Hash,
    io,
};

use super::time::clock::Clock;

pub trait RuntimeError: Debug + Display + Send + 'static {
    fn is_not_ready(&self) -> bool;
    fn io_error(&self) -> Option<&io::Error>;
    fn into_io_error(self) -> Option<io::Error>;
}

pub trait SocketAddr: Debug + Display + Clone + PartialEq + Eq + Hash + Send
where
    Self: From<std::net::SocketAddr>,
{
    fn network_address(&self) -> Option<&std::net::SocketAddr>;
}

pub trait Socket: Debug + Display + Clone + PartialEq + Eq + Hash + Send {}

pub trait Runtime: Send + 'static {
    type Error: RuntimeError;

    type Clock: Clock;

    type SocketAddr: SocketAddr;

    type Socket: Socket;

    fn get_clock(&mut self, id: Option<&str>) -> Self::Clock;

    fn get_default_clock(&mut self) -> Self::Clock {
        self.get_clock(None)
    }

    fn get_remote_address(&self, remote: Self::Socket) -> Result<Self::SocketAddr, Self::Error>;

    fn bind(&mut self, address: Self::SocketAddr) -> Result<Self::Socket, Self::Error>;

    fn connect(
        &mut self,
        socket: Self::Socket,
        address: Self::SocketAddr,
    ) -> Result<Self::Socket, Self::Error>;

    fn send(&mut self, socket: Self::Socket, buf: &[u8]) -> Result<(), Self::Error>;

    fn close(&mut self, socket: Self::Socket);
}
