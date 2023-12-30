#![allow(unused)]
use mio::{Poll, Token};
use rist_rs_types::traits::protocol::{self, Ctl, Events, ReadyFlags, IOV};
use std::collections::hash_map::{DefaultHasher, Entry};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{SocketAddr, UdpSocket};

use rist_rs_types::traits::runtime;

use crate::StdRuntime;

pub(crate) struct NetIo {
    poll: mio::Poll,
    socks: slab::Slab<mio::net::UdpSocket>,
    events: mio::Events,
}

fn map_ev_flags(ev: &mio::event::Event) -> ReadyFlags {
    let mut out = ReadyFlags::empty();
    out.set(ReadyFlags::Readable, ev.is_readable());
    out.set(ReadyFlags::Writable, ev.is_writable());
    out
}

impl NetIo {
    pub(crate) fn try_new() -> Result<Self, runtime::Error> {
        Ok(Self {
            poll: mio::Poll::new()?,
            socks: slab::Slab::new(),
            events: mio::Events::with_capacity(24),
        })
    }

    pub(crate) fn bind(&mut self, addr: SocketAddr) -> Result<crate::Socket, runtime::Error> {
        // bind the socket
        let mut sock = mio::net::UdpSocket::bind(addr)?;

        // add to slab
        let sock_id = self.socks.insert(sock);

        // register with poller
        self.poll.registry().register(
            self.socks.get_mut(sock_id).unwrap(),
            Token(sock_id),
            mio::Interest::WRITABLE | mio::Interest::READABLE,
        );

        let sock = crate::Socket::Net(sock_id);
        tracing::trace!(%addr, ?sock, "udp bound");
        Ok(sock)
    }

    pub(crate) fn connect(
        &mut self,
        socket: usize,
        addr: SocketAddr,
    ) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.connect(addr).map_err(From::from)
    }

    pub(crate) fn send(&mut self, socket: usize, buf: &[u8]) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.send(buf).map_err(From::from).map(|_| ())
    }

    pub(crate) fn send_to(
        &mut self,
        socket: usize,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.send_to(buf, addr).map_err(From::from).map(|_| ())
    }

    pub(crate) fn recv(&mut self, socket: usize, buf: &mut [u8]) -> Result<usize, runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.recv(buf).map_err(From::from)
    }

    pub(crate) fn recv_from(
        &mut self,
        socket: usize,
        buf: &mut [u8],
    ) -> Result<(usize, SocketAddr), runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.recv_from(buf).map_err(From::from)
    }

    pub(crate) fn map_event<C: protocol::Ctl>(
        &self,
        input: &mio::event::Event,
    ) -> protocol::IOV<StdRuntime, C> {
        match input {
            ev if ev.is_error() => {
                match self
                    .socks
                    .get(ev.token().0)
                    .map(mio::net::UdpSocket::take_error)
                {
                    Some(Ok(Some(err))) => IOV::Error(crate::Socket::Net(ev.token().0), err.into()),
                    any => IOV::Empty,
                }
            }
            ev if ev.is_readable() || ev.is_writable() => {
                IOV::Ready(crate::Socket::Net(ev.token().0), map_ev_flags(ev))
            }
            ev => {
                tracing::error!(?ev, "unexpected event");
                IOV::Empty
            }
        }
    }

    pub(crate) fn poll<C: Ctl>(&mut self, events: &mut Events<StdRuntime, C>) {
        // system poll
        if let Err(err) = self.poll.poll(&mut self.events, None) {
            tracing::error!(%err, "poll")
        }

        // copy events
        for ev in self.events.iter() {
            match self.map_event(ev) {
                IOV::Empty => {}
                ev => events.push(ev),
            }
        }
    }
}

#[allow(unused)]
mod test {
    use std::{
        net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
        str::{from_utf8, FromStr},
    };

    use rist_rs_types::traits::{
        packet,
        protocol::{self, ReadyFlags, IOV},
        runtime,
    };
    use socket2::SockAddr;

    use crate::{Socket, StdRuntime};

    use super::NetIo;

    #[derive(Debug, Clone)]
    struct TestCtl;

    type Events = protocol::Events<StdRuntime, TestCtl>;

    impl protocol::Ctl for TestCtl {
        type Error = ();

        type Output = ();

        fn start() -> Self {
            Self
        }

        fn shutdown() -> Self {
            Self
        }
    }

    fn poll_read_one(io: &mut NetIo) -> Vec<u8> {
        let mut buf = [0; 8192];
        let mut events = Events::new(1);
        loop {
            io.poll(&mut events);

            if let protocol::IOV::Ready(Socket::Net(s), flags) = events.pop().unwrap() {
                if flags.contains(ReadyFlags::Readable) {
                    let len = io.recv(s, &mut buf).unwrap();
                    return buf.split_at(len).0.into();
                }
            }
        }
    }

    fn poll_write_one(io: &mut NetIo, buf: &[u8], addr: SocketAddr) {
        let mut events = Events::new(1);
        loop {
            io.poll(&mut events);

            if let protocol::IOV::Ready(Socket::Net(s), flags) = events.pop().unwrap() {
                if flags.contains(ReadyFlags::Writable) {
                    io.send_to(s, buf, addr).unwrap();
                    return;
                }
            }
        }
    }

    #[test]
    fn test_rx() {
        let packet_out = [0u8];
        let bind_addr: SocketAddr = "127.0.0.1:10220".parse().unwrap();
        let mut io = NetIo::try_new().unwrap();
        io.bind(bind_addr).unwrap();
        let sender = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        sender.send_to(&packet_out, bind_addr);
        let packet_in = poll_read_one(&mut io);
        assert_eq!(packet_out, packet_in.as_slice());
    }

    #[test]
    fn test_tx() {
        let packet_out = [0u8];
        let bind_addr: SocketAddr = "127.0.0.1:10221".parse().unwrap();
        let mut io = NetIo::try_new().unwrap();
        let sock = UdpSocket::bind(bind_addr).unwrap();
        io.bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))
            .unwrap();
        poll_write_one(&mut io, &packet_out, bind_addr);
        let mut packet_in = [0u8; 1];
        sock.recv(&mut packet_in).unwrap();

        assert_eq!(packet_in, packet_out);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_tx_would_block() {
        let packet_out = [0u8; 1280];
        let send_adr: SocketAddr = "192.0.2.2:10221".parse().unwrap();
        let mut io = NetIo::try_new().unwrap();
        let sock = io
            .bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))
            .unwrap();
        if let crate::Socket::Net(sock_id) = sock {
            let mut events = Events::new(6);

            'writeable: loop {
                io.poll(&mut events);
                while let Some(ev) = events.pop() {
                    match ev {
                        IOV::Ready(s, flags) if flags.contains(ReadyFlags::Writable) => {
                            break 'writeable
                        }
                        ev => println!("{ev:?}"),
                    }
                }
            }

            loop {
                println!("write");
                match io.send_to(sock_id, &packet_out, send_adr) {
                    Err(runtime::Error::WouldBlock) => break,
                    Err(e) => panic!("{:?}", e),
                    Ok(_) => { /*println!("write")*/ }
                }
            }

            println!("wait writable");
            poll_write_one(&mut io, &packet_out, send_adr);
        } else {
            panic!("invalid socket type returned")
        }
    }
}
