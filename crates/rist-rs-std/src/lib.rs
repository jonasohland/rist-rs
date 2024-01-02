#![allow(unused)]

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::net::SocketAddr as StdSocketAddr;

use net::NetIo;
use rist_rs_types::traits::protocol::Protocol;
use rist_rs_types::traits::runtime;
use rist_rs_types::traits::time::clock::StdSystemClock;

mod net;

pub mod testing;
pub mod transport;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketAddr {
    NetworkAddress(StdSocketAddr),
}

impl runtime::SocketAddr for SocketAddr {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Socket {
    Empty,
    Net(usize),
}

impl runtime::Socket for Socket {}

impl Socket {
    pub fn empty() -> Self {
        Self::Empty
    }
}

pub struct StdRuntime {
    net: net::NetIo,
}

impl runtime::Runtime for StdRuntime {
    type Clock = StdSystemClock;

    type SocketAddr = SocketAddr;

    type Socket = Socket;

    fn get_clock(&mut self, _: Option<&str>) -> Self::Clock {
        StdSystemClock
    }

    fn bind(&mut self, address: Self::SocketAddr) -> Result<Self::Socket, runtime::Error> {
        match address {
            SocketAddr::NetworkAddress(net) => self.net.bind(net),
        }
    }

    fn connect(
        &mut self,
        local_sock_id: Self::Socket,
        address: Self::SocketAddr,
    ) -> Result<(), runtime::Error> {
        match local_sock_id {
            Socket::Empty => Err(runtime::Error::InvalidInput),
            Socket::Net(id) => {
                let SocketAddr::NetworkAddress(addr) = address;
                self.net.connect(id, addr)
            }
        }
    }

    fn send(&mut self, _socket: Self::Socket, _buf: &[u8]) -> Result<(), runtime::Error> {
        Ok(())
    }

    fn get_remote_address(&self, _: Self::Socket) -> Result<Self::SocketAddr, runtime::Error> {
        todo!()
    }

    fn close(&mut self, _socket: Self::Socket) {}

    fn send_to(
        &mut self,
        _socket: Self::Socket,
        _buf: &[u8],
        _address: Self::SocketAddr,
    ) -> Result<(), runtime::Error> {
        todo!()
    }

    fn recv(&mut self, _socket: Self::Socket, buf: &mut [u8]) -> Result<usize, runtime::Error> {
        todo!()
    }

    fn recv_from(
        &mut self,
        _socket: Self::Socket,
        _buf: &mut [u8],
    ) -> Result<(usize, Self::SocketAddr), runtime::Error> {
        todo!()
    }
}

impl StdRuntime {
    #[allow(clippy::new_without_default)]
    pub fn try_new() -> Result<Self, runtime::Error> {
        Ok(Self {
            net: NetIo::try_new()?,
        })
    }

    pub fn run<P: Protocol<Self>>(mut self, mut protocol: P) {}
}

impl Display for SocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketAddr::NetworkAddress(net) => Debug::fmt(net, f),
        }
    }
}

impl Display for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Socket::Empty => write!(f, "<empty>"),
            Socket::Net(_) => todo!(),
        }
    }
}

impl From<StdSocketAddr> for SocketAddr {
    fn from(value: StdSocketAddr) -> Self {
        Self::NetworkAddress(value)
    }
}
