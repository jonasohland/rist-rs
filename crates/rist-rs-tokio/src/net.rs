use core::task;

use std::{fmt::Debug, future::Future, net::SocketAddr, pin::Pin, task::Context};

use rist_rs_types::traits::{
    protocol::{self, Ctl, ReadyFlags, IOV},
    runtime,
};

use slab::Slab;
use tokio::net::UdpSocket;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    struct SockFlags: u8 {
        const WriteBlocked = 0x01;
        const ReadBlocked = 0x02;
    }
}

/// SocketState contains all necessary state to emulate edge-triggered behavior like you
/// would find it in epoll.
#[derive(Debug)]
struct SocketState {
    sock: UdpSocket,
    flags: SockFlags,
}

impl SocketState {
    fn new(sock: UdpSocket) -> Self {
        Self {
            sock,
            flags: SockFlags::ReadBlocked | SockFlags::WriteBlocked,
        }
    }

    fn get(&self) -> &UdpSocket {
        &self.sock
    }

    fn need_poll(&self) -> bool {
        self.flags
            .intersects(SockFlags::WriteBlocked | SockFlags::ReadBlocked)
    }

    fn on_send(&mut self, res: Result<usize, std::io::Error>) -> Result<(), runtime::Error> {
        match res.map_err(From::from).map(|_| ()) {
            Ok(_) => {
                self.flags.remove(SockFlags::WriteBlocked);
                Ok(())
            }
            Err(runtime::Error::WouldBlock) => {
                self.flags.insert(SockFlags::WriteBlocked);
                Err(runtime::Error::WouldBlock)
            }
            Err(e) => Err(e),
        }
    }

    fn on_recv<T: Debug>(&mut self, res: Result<T, std::io::Error>) -> Result<T, runtime::Error> {
        match res.map_err(From::from) {
            Ok(v) => {
                self.flags.remove(SockFlags::ReadBlocked);
                Ok(v)
            }
            Err(runtime::Error::WouldBlock) => {
                self.flags.insert(SockFlags::ReadBlocked);
                Err(runtime::Error::WouldBlock)
            }
            Err(e) => Err(e),
        }
    }

    fn try_send(&mut self, buf: &[u8]) -> Result<(), runtime::Error> {
        self.on_send(self.sock.try_send(buf))
    }

    fn try_send_to(&mut self, buf: &[u8], addr: SocketAddr) -> Result<(), runtime::Error> {
        self.on_send(self.sock.try_send_to(buf, addr))
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> Result<usize, runtime::Error> {
        self.on_recv(self.sock.try_recv(buf))
    }

    fn try_recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), runtime::Error> {
        self.on_recv(self.sock.try_recv_from(buf))
    }

    fn poll<C: Ctl>(
        &mut self,
        id: usize,
        cx: &mut task::Context<'_>,
    ) -> Option<IOV<crate::Runtime, C>> {
        if !self.need_poll() {
            None
        } else {
            let mut readiness = ReadyFlags::empty();
            if self.flags.contains(SockFlags::ReadBlocked) {
                println!("poll rcv");
                match self.sock.poll_recv_ready(cx) {
                    task::Poll::Ready(Ok(_)) => {
                        self.flags.remove(SockFlags::ReadBlocked);
                        readiness.set(ReadyFlags::Readable, true)
                    }
                    task::Poll::Ready(Err(_)) => {}
                    task::Poll::Pending => {}
                }
            }
            if self.flags.contains(SockFlags::WriteBlocked) {
                match self.sock.poll_send_ready(cx) {
                    task::Poll::Ready(Ok(_)) => {
                        self.flags.remove(SockFlags::WriteBlocked);
                        readiness.set(ReadyFlags::Writable, true)
                    }
                    task::Poll::Ready(Err(_)) => {}
                    task::Poll::Pending => {}
                }
            }
            if !readiness.is_empty() {
                Some(protocol::IOV::Ready(crate::Socket::Net(id), readiness))
            } else {
                None
            }
        }
    }
}

#[derive(Default)]
pub struct NetIo {
    socks: Slab<SocketState>,
}

impl NetIo {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<crate::Socket, runtime::Error> {
        let waker = noop_waker::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let mut future = Box::pin(UdpSocket::bind(addr));
        let sock = match Pin::new(&mut future).poll(&mut cx) {
            task::Poll::Ready(res) => res?,
            task::Poll::Pending => Err(runtime::Error::Str(
                "tokio bind() pending after poll, but polling not possible",
            ))?,
        };
        Ok(crate::Socket::Net(
            self.socks.insert(SocketState::new(sock)),
        ))
    }

    pub fn connect(&mut self, sock: usize, addr: SocketAddr) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(sock)
            .ok_or(runtime::Error::InvalidInput)?;

        let mut future_boxed = Box::pin(sock.get().connect(addr));

        let waker = noop_waker::noop_waker();
        let mut cx = Context::from_waker(&waker);

        match Pin::new(&mut future_boxed).poll(&mut cx) {
            task::Poll::Ready(res) => res.map_err(From::from),
            task::Poll::Pending => Err(runtime::Error::Str(
                "tokio connect() pending after poll, but polling not possible",
            ))?,
        }
    }

    pub fn send(&mut self, sock: usize, buf: &[u8]) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(sock)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.try_send(buf)
    }

    pub fn send_to(
        &mut self,
        sock: usize,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Result<(), runtime::Error> {
        let sock = self
            .socks
            .get_mut(sock)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.try_send_to(buf, addr)
    }

    pub fn recv(&mut self, socket: usize, buf: &mut [u8]) -> Result<usize, runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.try_recv(buf)
    }

    pub fn recv_from(
        &mut self,
        socket: usize,
        buf: &mut [u8],
    ) -> Result<(usize, SocketAddr), runtime::Error> {
        let sock = self
            .socks
            .get_mut(socket)
            .ok_or(runtime::Error::InvalidInput)?;
        sock.try_recv_from(buf)
    }

    pub fn poll<C: Ctl>(
        &mut self,
        cx: &mut task::Context<'_>,
        events: &mut protocol::Events<crate::Runtime, C>,
    ) {
        for (id, sock) in &mut self.socks {
            if let Some(ev) = sock.poll(id, cx) {
                events.push(ev)
            }
        }
    }
}

#[allow(unused)]
mod test {
    use std::future::Future;

    use rist_rs_types::traits::{
        protocol::{self, ReadyFlags, IOV},
        runtime,
    };

    use super::NetIo;

    #[derive(Debug, Clone)]
    struct TestCtl;

    type Events = protocol::Events<crate::Runtime, TestCtl>;

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

    struct EventsFuture<'a>(&'a mut NetIo);

    impl<'a> Future for EventsFuture<'a> {
        type Output = Events;

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            let mut events = Events::new(24);
            self.0.poll(cx, &mut events);
            if events.is_empty() {
                std::task::Poll::Pending
            } else {
                std::task::Poll::Ready(events)
            }
        }
    }

    #[allow(unused)]
    async fn test_rx() {
        let mut io = NetIo::new();

        let sock = io
            .bind(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                19021,
            )))
            .unwrap();

        if let crate::Socket::Net(sock_id) = sock {
            let mut buf = [0u8; 1500];

            loop {
                'poll: loop {
                    let mut events = EventsFuture(&mut io).await;
                    while let Some(ev) = events.pop() {
                        if let IOV::Ready(_, flags) = ev {
                            if flags.contains(ReadyFlags::Readable) {
                                loop {
                                    match io.recv(sock_id, &mut buf) {
                                        Ok(len) => {
                                            println!("{len}")
                                        }
                                        Err(runtime::Error::WouldBlock) => {
                                            println!("WouldBlock");
                                            break 'poll;
                                        }
                                        Err(e) => panic!("{:?}", e),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            panic!("invalid socket type returned")
        }
    }
}
