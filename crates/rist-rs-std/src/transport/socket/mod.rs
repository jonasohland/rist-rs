#![allow(unused)]
use std::{
    borrow::Borrow,
    io::{self, Read},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

use rist_rs_types::traits::io::{
    ReadNonBlocking, ReceiveFromNonBlocking, ReceiveNonBlocking, SendNonBlocking,
    SendToNonBlocking, WriteNonBlocking,
};

/// Non-blocking UDP socket. Abstracts await handling of the WouldBlock error, and removes
/// blocking DNS lockup from the ::bind call
pub struct NonBlockingUdpSocket(UdpSocket);

impl NonBlockingUdpSocket {
    fn transform_would_block<R>(r: Result<R, io::Error>) -> Option<Result<R, io::Error>> {
        match r {
            Ok(t) => Some(Ok(t)),
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => None,
                _ => Some(Err(e)),
            },
        }
    }

    /// Wrap an existing UdpSocket, the socket will be turned into a non-blocking socket by this call
    pub fn wrap(socket: UdpSocket) -> Result<NonBlockingUdpSocket, io::Error> {
        socket.set_nonblocking(true)?;
        Ok(Self(socket))
    }

    /// Create a bound socket
    pub fn bind(address: impl Borrow<SocketAddr>) -> Result<NonBlockingUdpSocket, io::Error> {
        let mut socket = UdpSocket::bind(address.borrow())?;
        socket.set_nonblocking(true)?;
        Ok(Self(socket))
    }

    /// Connect a socket to a remote address
    pub fn connect(&mut self, address: impl Borrow<SocketAddr>) -> Result<(), io::Error> {
        self.0.connect(address.borrow())
    }

    /// Get a reference to the wrapped UdpSocket
    pub fn inner(&self) -> &UdpSocket {
        &self.0
    }

    /// Get a mutable reference to the wrapped socket
    pub fn inner_mut(&mut self) -> &mut UdpSocket {
        &mut self.0
    }

    /// Convert to the wrapped socket. The socket will still be non-blocking
    pub fn into_inner(self) -> UdpSocket {
        self.0
    }

    /// Convert into a blocking socket
    pub fn into_blocking(self) -> Result<UdpSocket, io::Error> {
        self.0.set_nonblocking(false)?;
        Ok(self.0)
    }
}

impl SendNonBlocking for NonBlockingUdpSocket {
    type Error = io::Error;

    fn try_send(&mut self, buf: &[u8]) -> Option<Result<usize, Self::Error>> {
        Self::transform_would_block(self.0.send(buf))
    }
}

impl SendToNonBlocking for NonBlockingUdpSocket {
    type Error = io::Error;
    type Address = SocketAddr;

    fn try_send_to<A: Borrow<Self::Address>>(
        &mut self,
        buf: &[u8],
        address: A,
    ) -> Option<Result<usize, Self::Error>> {
        Self::transform_would_block(self.0.send_to(buf, address.borrow()))
    }
}

impl ReceiveNonBlocking for NonBlockingUdpSocket {
    type Error = io::Error;

    fn try_recv(&mut self, buf: &mut [u8]) -> Option<Result<usize, Self::Error>> {
        Self::transform_would_block(self.0.recv(buf))
    }
}

impl ReceiveFromNonBlocking for NonBlockingUdpSocket {
    type Error = io::Error;
    type Address = SocketAddr;

    fn try_recv_from(
        &mut self,
        buf: &mut [u8],
    ) -> Option<Result<(usize, Self::Address), Self::Error>> {
        Self::transform_would_block(self.0.recv_from(buf))
    }
}

#[allow(unused)]
mod test {

    use super::*;
    use std::str::{from_utf8, FromStr};

    #[test]
    fn bind() {
        drop(NonBlockingUdpSocket::bind(SocketAddr::from_str("0.0.0.0:0").unwrap()).unwrap());
    }

    #[test]
    fn read_no_data() {
        let mut socket =
            NonBlockingUdpSocket::bind(SocketAddr::from_str("0.0.0.0:0").unwrap()).unwrap();
        let mut buf = [];
        // should not block and return no data
        assert!(matches!(socket.try_recv(&mut buf), None));
    }

    #[test]
    fn transmit_non_blocking() {
        let rx_listen = SocketAddr::from_str("0.0.0.0:37702").unwrap();
        let mut tx =
            NonBlockingUdpSocket::bind(SocketAddr::from_str("0.0.0.0:0").unwrap()).unwrap();
        let mut rx = NonBlockingUdpSocket::bind(rx_listen).unwrap();
        tx.connect(rx_listen).unwrap();
        let buf = "Hello!".as_bytes();
        let mut rxbuf = vec![0u8; buf.len()];
        match tx.try_send(buf) {
            Some(r) => match r {
                Ok(len) => assert_eq!(len, buf.len()),
                Err(e) => panic!("{}", e),
            },
            _ => panic!(),
        }
        match rx.try_recv(&mut rxbuf) {
            Some(r) => match r {
                Ok(s) => {
                    assert_eq!(s, buf.len());
                    assert_eq!(from_utf8(&rxbuf).unwrap(), "Hello!")
                }
                Err(e) => panic!("{}", e),
            },
            _ => panic!(),
        }
        assert!(matches!(rx.try_recv(&mut rxbuf), None));
    }
}
