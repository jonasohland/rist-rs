use std::fmt::{Debug, Display};

use rist_rs_types::traits::{runtime, time::clock::StdSystemClock};

pub mod net;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketAddr {
    NetworkAddress(std::net::SocketAddr),
}

impl Display for SocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketAddr::NetworkAddress(net) => Debug::fmt(net, f),
        }
    }
}

impl From<std::net::SocketAddr> for SocketAddr {
    fn from(value: std::net::SocketAddr) -> Self {
        Self::NetworkAddress(value)
    }
}

impl runtime::SocketAddr for SocketAddr {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Socket {
    Empty,
    Net(usize),
}

impl Display for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Socket::Empty => write!(f, "<empty>"),
            Socket::Net(_) => todo!(),
        }
    }
}

impl runtime::Socket for Socket {}

impl Socket {
    pub fn empty() -> Self {
        Self::Empty
    }
}

#[derive(Debug)]
pub struct Runtime {}

impl runtime::Runtime for Runtime {
    type Clock = StdSystemClock;

    type SocketAddr = SocketAddr;

    type Socket = Socket;

    fn get_clock(&mut self, _id: Option<&str>) -> Self::Clock {
        todo!()
    }

    fn get_remote_address(
        &self,
        _remote: Self::Socket,
    ) -> Result<Self::SocketAddr, rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn bind(
        &mut self,
        _address: Self::SocketAddr,
    ) -> Result<Self::Socket, rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn connect(
        &mut self,
        _socket: Self::Socket,
        _address: Self::SocketAddr,
    ) -> Result<(), rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn send(
        &mut self,
        _socket: Self::Socket,
        _buf: &[u8],
    ) -> Result<(), rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn send_to(
        &mut self,
        _socket: Self::Socket,
        _buf: &[u8],
        _address: Self::SocketAddr,
    ) -> Result<(), rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn recv(
        &mut self,
        _socket: Self::Socket,
        _buf: &mut [u8],
    ) -> Result<usize, rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn recv_from(
        &mut self,
        _socket: Self::Socket,
        _buf: &mut [u8],
    ) -> Result<(usize, Self::SocketAddr), rist_rs_types::traits::runtime::Error> {
        todo!()
    }

    fn close(&mut self, _socket: Self::Socket) {
        todo!()
    }
}
