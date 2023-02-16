#![allow(unused)]
use std::collections::hash_map::{DefaultHasher, Entry};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{SocketAddr, UdpSocket};

use crate::{IoEvent, IoEventKind};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SocketId(pub(crate) usize);

impl SocketId {
    pub fn empty() -> SocketId {
        SocketId(usize::MAX)
    }

    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}

impl Debug for SocketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.hash())
    }
}

impl Display for SocketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.hash())
    }
}

struct LocalSocket {
    socket: UdpSocket,
    remotes: HashMap<SocketAddr, usize>,
}

struct RemoteSocket {
    local_socket_id: usize,
    remote_address: SocketAddr,
}

pub struct Sockets {
    num_local_sockets: usize,
    num_remote_sockets: usize,
    sockets: Vec<Option<LocalSocket>>,
    remote_sockets: Vec<Option<RemoteSocket>>,
}

impl Sockets {
    const SOCK_INDEX_PIVOT: usize = usize::MAX / 2;

    pub fn new() -> Self {
        Self {
            num_local_sockets: 0,
            num_remote_sockets: 0,
            sockets: Default::default(),
            remote_sockets: Default::default(),
        }
    }

    fn bind_non_blocking_socket(address: SocketAddr) -> Result<UdpSocket, io::Error> {
        let socket = socket2::Socket::from(UdpSocket::bind(address)?);
        socket.set_nonblocking(true)?;
        Ok(socket.into())
    }

    fn update_active_socket_count(&mut self) {
        self.num_remote_sockets = self.remote_sockets.iter().filter(|s| s.is_some()).count();
        self.num_local_sockets = self.sockets.iter().filter(|s| s.is_some()).count()
    }

    fn close_remote_socket(&mut self, socket: usize) {
        let idx = socket - Self::SOCK_INDEX_PIVOT;
        if idx <= self.remote_sockets.len() {
            match self.remote_sockets[idx].take() {
                Some(remote_socket) => match &mut self.sockets[remote_socket.local_socket_id] {
                    Some(sock) => {
                        tracing::trace!(remote_socket_address = %remote_socket.remote_address, remote_socket_index = idx, "removing remote socket entry");
                        sock.remotes.remove(&remote_socket.remote_address);
                    }
                    None => {
                        tracing::warn!(socket, "remote socket leaked");
                    }
                },
                None => {
                    tracing::warn!(socket, "orphaned socket closed");
                }
            }
        }
    }

    fn close_local_socket(&mut self, socket: usize) {
        if socket <= self.sockets.len() {
            match self.sockets[socket].take() {
                Some(local_socket) => {
                    for (_, remote_socket) in local_socket.remotes {
                        self.remote_sockets[remote_socket - Self::SOCK_INDEX_PIVOT].take();
                    }
                }
                None => todo!(),
            }
        }
    }

    fn reserve_socket<S>(sockets: &mut Vec<Option<S>>) -> usize {
        let mut idx = 0usize;
        loop {
            if idx == sockets.len() {
                sockets.push(None);
                break;
            }
            if sockets[idx].is_none() {
                break;
            }
            idx += 1
        }
        idx
    }

    pub fn add(&mut self, socket: UdpSocket) -> io::Result<SocketId> {
        let socket = socket2::Socket::from(socket);
        socket.set_nonblocking(true)?;
        let idx = Self::reserve_socket(&mut self.sockets);
        self.sockets[idx] = Some(LocalSocket {
            socket: socket.into(),
            remotes: Default::default(),
        });
        self.update_active_socket_count();
        Ok(SocketId(idx))
    }

    pub fn bind(&mut self, address: SocketAddr) -> io::Result<SocketId> {
        let socket = Self::bind_non_blocking_socket(address)?;
        let mut idx = Self::reserve_socket(&mut self.sockets);
        self.sockets[idx] = Some(LocalSocket {
            socket,
            remotes: Default::default(),
        });
        self.update_active_socket_count();
        Ok(SocketId(idx))
    }

    pub fn close(&mut self, socket: SocketId) {
        if socket.0 >= Self::SOCK_INDEX_PIVOT {
            self.close_remote_socket(socket.0)
        } else {
            self.close_local_socket(socket.0)
        }
        self.update_active_socket_count()
    }

    pub fn connect(
        &mut self,
        local_socket_id: SocketId,
        remote_address: SocketAddr,
    ) -> io::Result<SocketId> {
        let local_socket_id = local_socket_id.0;
        match &mut self.sockets[local_socket_id] {
            None => Err(io::Error::from(io::ErrorKind::InvalidInput)),
            Some(socket_entry) => {
                let idx = Self::reserve_socket(&mut self.remote_sockets);
                let remote_socket_id = idx + Self::SOCK_INDEX_PIVOT;
                tracing::trace!(%remote_address, remote_socket_index = idx, "insert new remote socket entry");
                self.remote_sockets[idx] = Some(RemoteSocket {
                    local_socket_id,
                    remote_address,
                });
                socket_entry
                    .remotes
                    .insert(remote_address, remote_socket_id);
                self.update_active_socket_count();
                Ok(SocketId(remote_socket_id))
            }
        }
    }

    pub fn send_non_blocking(&self, remote_socket_id: SocketId, buf: &[u8]) -> io::Result<()> {
        let remote_socket_id = remote_socket_id.0;
        match self.remote_sockets[remote_socket_id - Self::SOCK_INDEX_PIVOT]
            .as_ref()
            .and_then(|remote| {
                self.sockets[remote.local_socket_id]
                    .as_ref()
                    .map(|socket| (remote.remote_address, &socket.socket))
            }) {
            Some((addr, sock)) => sock.send_to(buf, addr).map(|_| ()),
            None => Err(io::Error::from(io::ErrorKind::InvalidInput)),
        }
    }

    pub fn poll_events(&mut self, events: &mut [IoEvent]) {
        let reads_per_sock =
            1.max((events.len() / self.num_local_sockets.max(1)) - self.num_remote_sockets);
        let mut events_index = 0;
        let remote_sockets = &mut self.remote_sockets;
        for (local_socket_id, opt_socket_entry) in self.sockets.iter_mut().enumerate() {
            if let Some(socket_entry) = opt_socket_entry {
                for _ in 0..reads_per_sock {
                    if events_index == events.len() {
                        // event buffer already filled
                        return;
                    }
                    let event = &mut events[events_index];
                    match socket_entry.socket.recv_from(&mut event.buf) {
                        Ok((len, remote_address)) => {
                            event.len = len;
                            event.socket = SocketId(local_socket_id).into();
                            match socket_entry.remotes.entry(remote_address) {
                                Entry::Occupied(entry) => {
                                    event.kind =
                                        IoEventKind::Readable(SocketId(*entry.get()).into())
                                }
                                Entry::Vacant(entry) => {
                                    let remote_socket_index = Self::reserve_socket(remote_sockets);
                                    tracing::trace!(%remote_address, remote_socket_index, "insert new remote socket entry");
                                    let remote_socket_id =
                                        remote_socket_index + Self::SOCK_INDEX_PIVOT;
                                    entry.insert(remote_socket_id);
                                    remote_sockets[remote_socket_index] = Some(RemoteSocket {
                                        local_socket_id,
                                        remote_address,
                                    });
                                    event.kind = IoEventKind::Accept(
                                        remote_address.into(),
                                        SocketId(remote_socket_id).into(),
                                    )
                                }
                            }
                        }
                        Err(error) => {
                            if error.kind() == io::ErrorKind::WouldBlock {
                                break;
                            } else {
                                event.len = 0;
                                event.socket = SocketId(local_socket_id).into();
                                event.kind = IoEventKind::Error(error.into())
                            }
                        }
                    }
                    events_index += 1;
                }
            }
        }
        for (i, socket) in self
            .remote_sockets
            .iter()
            .enumerate()
            .filter_map(|(i, remote_socket)| remote_socket.as_ref().map(|s| (i, s)))
        {
            if events_index == events.len() {
                // event buffer already filled
                return;
            }
            events[events_index].len = 0;
            events[events_index].socket = SocketId(socket.local_socket_id).into();
            events[events_index].kind =
                IoEventKind::Writable(SocketId(Self::SOCK_INDEX_PIVOT + i).into());
            events_index += 1;
        }
        for event in events.iter_mut().skip(events_index) {
            event.reset();
        }
    }
}

#[allow(unused)]
mod test {

    use super::{SocketId, Sockets};
    use crate::testing::{self, BusyLoopTimeout};
    use crate::{IoEvent, IoEventKind, Socket};
    use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
    use std::time::Duration;

    pub fn expect_accept_event(events: &[IoEvent]) -> Option<&IoEvent> {
        for event in events {
            if let IoEventKind::Accept(remote_address, remote_socket_id) = event.kind {
                return Some(event);
            }
        }
        None
    }

    #[test]
    fn test_bind_connect_close() {
        let mut sockets = Sockets::new();
        let (port, socket) = testing::get_localhost_bound_socket();
        drop(socket);
        let socket = sockets
            .bind(testing::sock_addr_localhost(port))
            .expect("bind operation failed");
        sockets.close(socket);
        UdpSocket::bind(testing::sock_addr_localhost(port)).expect("port was not closed correctly");
    }

    #[test]
    fn bind_accept() {
        let mut events = IoEvent::allocate(1, 24);
        let mut sockets = Sockets::new();
        let (port, socket) = testing::get_localhost_bound_socket();
        let socket = sockets.add(socket).unwrap();
        let (test_port, test_socket) = testing::get_localhost_bound_socket();

        // send a message to the listening socket
        test_socket
            .send_to(&[0x00], testing::sock_addr_localhost(port))
            .unwrap();
        let mut timeout = BusyLoopTimeout::new(Duration::from_secs(5));
        loop {
            sockets.poll_events(&mut events);
            if let IoEventKind::Accept(remote_address, remote_socket_id) = &events[0].kind {
                // received an 'Accept' event
                assert_eq!(
                    *remote_address,
                    testing::sock_addr_localhost(test_port).into()
                );
                assert_eq!(events[0].socket, socket.into());
                assert_eq!(events[0].len, 1);

                if let Socket::NetworkSocket(socket) = remote_socket_id {
                    // close the remote socket
                    sockets.close(*socket);
                } else {
                    panic!("invalid socket returned in io event")
                }
                break;
            }

            if timeout.sleep() {
                panic!("timeout")
            }
        }

        // send another message
        test_socket
            .send_to(&[0x00], testing::sock_addr_localhost(port))
            .unwrap();

        // since the remote socket was closed we expect a another Accept event
        let mut timeout = BusyLoopTimeout::new(Duration::from_secs(5));
        loop {
            sockets.poll_events(&mut events);
            if let IoEventKind::Accept(remote_address, remote_socket_id) = &events[0].kind {
                assert_eq!(
                    *remote_address,
                    testing::sock_addr_localhost(test_port).into()
                );
                assert_eq!(events[0].socket, socket.into());
                assert_eq!(events[0].len, 1);
                break;
            }

            if timeout.sleep() {
                panic!("timeout")
            }
        }

        // make sure no more events are returned and the previous event was cleared
        sockets.poll_events(&mut events);
        assert!(matches!(
            events[0].kind,
            IoEventKind::Writable(_) | IoEventKind::None
        ));
    }

    #[test]
    fn accept_read() {
        let mut events = IoEvent::allocate(1, 24);
        let mut sockets = Sockets::new();
        let (port, socket) = testing::get_localhost_bound_socket();
        let socket = sockets.add(socket).unwrap();
        let mut remote_socket = SocketId::empty();
        let (test_port, test_socket) = testing::get_localhost_bound_socket();

        // send a message to the listening socket
        test_socket
            .send_to(&[0x00], testing::sock_addr_localhost(port))
            .unwrap();
        let mut timeout = BusyLoopTimeout::new(Duration::from_secs(5));
        loop {
            sockets.poll_events(&mut events);
            if let IoEventKind::Accept(remote_address, remote_socket_id) = &events[0].kind {
                // received an 'Accept' event
                assert_eq!(
                    *remote_address,
                    testing::sock_addr_localhost(test_port).into()
                );
                assert_eq!(events[0].socket, socket.into());
                assert_eq!(events[0].len, 1);
                match *remote_socket_id {
                    Socket::NetworkSocket(remote_socket_id) => {
                        remote_socket = remote_socket_id;
                    }
                    _ => panic!("invalid socket returned in io event"),
                }
                break;
            }

            if timeout.sleep() {
                panic!("timeout")
            }
        }

        // send another message
        test_socket
            .send_to(&[0x00], testing::sock_addr_localhost(port))
            .unwrap();

        // since the remote socket was closed we expect a another Accept event
        let mut timeout = BusyLoopTimeout::new(Duration::from_secs(5));
        loop {
            sockets.poll_events(&mut events);
            if let IoEventKind::Readable(remote_socket_id) = &events[0].kind {
                assert_eq!(*remote_socket_id, remote_socket.into());
                assert_eq!(events[0].socket, socket.into());
                assert_eq!(events[0].len, 1);
                break;
            }

            if timeout.sleep() {
                panic!("timeout")
            }
        }
    }

    #[test]
    fn accept_reply() {
        let mut events = IoEvent::allocate(1, 24);
        let mut sockets = Sockets::new();
        let (port, socket) = testing::get_localhost_bound_socket();
        let socket = sockets.add(socket).unwrap();
        let (ex_port, ex_socket) = testing::get_localhost_bound_socket();
        ex_socket
            .send_to(&[0x00], testing::sock_addr_localhost(port))
            .unwrap();
        let mut timeout = BusyLoopTimeout::new(Duration::from_secs(5));
        loop {
            sockets.poll_events(&mut events);
            if let IoEventKind::Accept(remote_address, remote_socket_id) = &events[0].kind {
                assert_eq!(
                    *remote_address,
                    testing::sock_addr_localhost(ex_port).into()
                );
                assert_eq!(events[0].socket, socket.into());
                assert_eq!(events[0].len, 1);
                match *remote_socket_id {
                    Socket::NetworkSocket(remote_socket_id) => {
                        sockets
                            .send_non_blocking(remote_socket_id, &[0xff])
                            .expect("send failed");
                    }
                    _ => panic!("invalid socket type returned in io event"),
                }
                break;
            }
            if timeout.sleep() {
                panic!("timeout")
            }
        }

        let mut buf = [0u8; 24];
        let (len, addr) = ex_socket.recv_from(&mut buf).expect("receive failed");
        assert_eq!(len, 1);
        assert_eq!(addr, testing::sock_addr_localhost(port));
    }
}
