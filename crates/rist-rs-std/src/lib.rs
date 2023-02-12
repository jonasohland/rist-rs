// #![allow(unused)]

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io;
use std::net::SocketAddr as StdSocketAddr;
use std::time::Duration;

use rist_rs_types::traits::protocol::{Ctl, Protocol};
use rist_rs_types::traits::runtime::{self, Runtime, RuntimeError};
use rist_rs_types::traits::time::clock::StdSystemClock;
use rist_rs_util::collections::static_vec::StaticVec;

mod net;

pub mod testing;
pub mod transport;

use net::Sockets as NetworkSockets;

#[derive(Debug)]
pub enum StdRuntimeError {
    UnknownSocket(u32),
    IOE(std::io::Error),
    Panic(&'static str),
}

impl Display for StdRuntimeError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl RuntimeError for StdRuntimeError {
    fn is_not_ready(&self) -> bool {
        if let StdRuntimeError::IOE(err) = self {
            return err.kind() == std::io::ErrorKind::WouldBlock;
        }
        false
    }

    fn io_error(&self) -> Option<&io::Error> {
        match self {
            StdRuntimeError::IOE(e) => Some(e),
            _ => None,
        }
    }

    fn into_io_error(self) -> Option<io::Error> {
        match self {
            StdRuntimeError::IOE(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketAddr {
    NetworkAddress(StdSocketAddr),
}

impl runtime::SocketAddr for SocketAddr {
    fn network_address(&self) -> Option<&std::net::SocketAddr> {
        #[allow(unreachable_patterns)]
        match self {
            SocketAddr::NetworkAddress(addr) => Some(addr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Socket {
    NetworkSocket(net::SocketId),
    Empty,
}

impl runtime::Socket for Socket {}

impl Socket {
    pub fn empty() -> Self {
        Self::Empty
    }
}

pub enum IoEventKind {
    Accept(SocketAddr, Socket),
    Readable(Socket),
    Writable(Socket),
    Error(StdRuntimeError),
    None,
}

pub struct IoEvent {
    pub socket: Socket,
    pub kind: IoEventKind,
    pub buf: StaticVec<u8>,
    pub len: usize,
}

impl IoEvent {
    pub fn allocate(num: usize, buf_len: usize) -> Vec<IoEvent> {
        (0..num)
            .map(|_| IoEvent {
                kind: IoEventKind::None,
                len: 0,
                buf: StaticVec::new(buf_len),
                socket: Socket::empty(),
            })
            .collect()
    }

    pub fn reset(&mut self) {
        self.kind = IoEventKind::None;
        self.len = 0;
    }

    pub fn is_none(&self) -> bool {
        matches!(self.kind, IoEventKind::None)
    }
}

pub struct StdRuntime {
    network_sockets: NetworkSockets,
}

impl Runtime for StdRuntime {
    type Error = StdRuntimeError;

    type Clock = StdSystemClock;

    type SocketAddr = SocketAddr;

    type Socket = Socket;

    fn get_clock(&mut self, _: Option<&str>) -> Self::Clock {
        StdSystemClock
    }

    fn bind(&mut self, address: Self::SocketAddr) -> Result<Self::Socket, Self::Error> {
        match address {
            SocketAddr::NetworkAddress(address) => self
                .network_sockets
                .bind(address)
                .map_err(StdRuntimeError::IOE)
                .map(Into::into),
        }
    }

    fn connect(
        &mut self,
        local_sock_id: Self::Socket,
        address: Self::SocketAddr,
    ) -> Result<Self::Socket, Self::Error> {
        match address {
            SocketAddr::NetworkAddress(address) => {
                if let Socket::NetworkSocket(socket) = local_sock_id {
                    self.network_sockets
                        .connect(socket, address)
                        .map_err(StdRuntimeError::IOE)
                        .map(Into::into)
                } else {
                    Err(StdRuntimeError::UnknownSocket(8))
                }
            }
        }
    }

    fn send(&mut self, socket: Self::Socket, buf: &[u8]) -> Result<(), Self::Error> {
        match socket {
            Socket::NetworkSocket(socket) => self
                .network_sockets
                .send_non_blocking(socket, buf)
                .map_err(StdRuntimeError::IOE),
            Socket::Empty => Ok(()),
        }
    }

    fn get_remote_address(&self, _: Self::Socket) -> Result<Self::SocketAddr, Self::Error> {
        todo!()
    }

    fn close(&mut self, socket: Self::Socket) {
        match socket {
            Socket::NetworkSocket(socket) => self.network_sockets.close(socket),
            Socket::Empty => {}
        }
    }
}

impl StdRuntime {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            network_sockets: NetworkSockets::new(),
        }
    }

    pub fn run_protocol<P: Protocol<Self>>(mut self, mut protocol: P) {
        let mut events = IoEvent::allocate(24, 1500);
        protocol.ctl(&mut self, <P::Ctl as Ctl>::start()).unwrap();
        loop {
            self.network_sockets.poll_events(&mut events);
            for event in events
                .iter_mut()
                .take_while(|e| !matches!(e.kind, IoEventKind::None))
            {
                match &event.kind {
                    IoEventKind::None => unreachable!(),
                    IoEventKind::Accept(remote_address, remote_socket_id) => {
                        protocol.accept(
                            &mut self,
                            event.socket,
                            *remote_socket_id,
                            *remote_address,
                        );
                    }
                    IoEventKind::Readable(remote_socket_id) => {
                        protocol.receive(
                            &mut self,
                            *remote_socket_id,
                            event.buf.split_at(event.len).0,
                        );
                    }
                    IoEventKind::Writable(socket) => {
                        protocol.writeable(&mut self, *socket);
                    }
                    IoEventKind::Error(error) => {
                        tracing::error!(?error, "socket error");
                        if let Socket::NetworkSocket(socket) = event.socket {
                            self.network_sockets.close(socket)
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
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
            Socket::NetworkSocket(net) => Display::fmt(net, f),
            Socket::Empty => write!(f, "<empty>"),
        }
    }
}

impl From<std::io::Error> for StdRuntimeError {
    fn from(value: std::io::Error) -> Self {
        StdRuntimeError::IOE(value)
    }
}

impl From<StdSocketAddr> for SocketAddr {
    fn from(value: StdSocketAddr) -> Self {
        Self::NetworkAddress(value)
    }
}

impl From<net::SocketId> for Socket {
    fn from(value: net::SocketId) -> Self {
        Self::NetworkSocket(value)
    }
}
