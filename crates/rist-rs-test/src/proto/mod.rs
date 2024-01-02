#![allow(unused)]

use std::{
    collections::{hash_map::Entry, HashMap},
    net::SocketAddr,
    time::Duration,
};

use rist_rs_types::traits::{
    protocol::{Ctl, Protocol, ProtocolEvent},
    runtime::{self, Event, Runtime, SocketAddr as TSocketAddr},
    time::clock::{Clock, TimePoint},
};

#[derive(Debug, Clone)]
pub struct SimpleProtoCtl;

impl Ctl for SimpleProtoCtl {
    type Error = ();
    type Output = ();
    fn start() -> Self {
        Self
    }
    fn shutdown() -> Self {
        Self
    }
}

struct Peer<R>
where
    R: Runtime,
{
    last_contact: <R::Clock as Clock>::TimePoint,
    last_send: <R::Clock as Clock>::TimePoint,
    stale: bool,
    blocked: bool,
    address: R::SocketAddr,
}

impl<R> Peer<R>
where
    R: Runtime,
{
    fn new(now: <R::Clock as Clock>::TimePoint, address: R::SocketAddr) -> Self {
        Self {
            last_contact: now,
            last_send: now,
            blocked: false,
            stale: false,
            address,
        }
    }
}

pub struct SimpleProto<R>
where
    R: Runtime,
{
    local_socket: R::Socket,
    start_peers: Option<Vec<SocketAddr>>,
    peers: HashMap<R::Socket, Peer<R>>,
    peer_list_message: Vec<u8>,
}

impl<R> SimpleProto<R>
where
    R: Runtime,
{
    pub fn new(socket: R::Socket, start_peers: Vec<SocketAddr>) -> Self {
        Self {
            local_socket: socket,
            start_peers: Some(start_peers),
            peers: Default::default(),
            peer_list_message: bincode::serialize::<Vec<SocketAddr>>(&vec![]).unwrap(),
        }
    }

    fn cleanup_dead_peers(&mut self, rt: &mut R, now: Option<<R::Clock as Clock>::TimePoint>) {
        let now = now.unwrap_or_else(|| rt.get_default_clock().now());
        let keys = self.peers.keys().cloned().collect::<Vec<_>>();
        let mut updated = false;
        for key in keys {
            if let Entry::Occupied(mut entry) = self.peers.entry(key) {
                if now
                    .duration_since(entry.get().last_contact)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    > Duration::from_secs(10)
                {
                    tracing::info!(remote_socket = %entry.key(), remote_address = %entry.get().address, "peer timed out");
                    rt.close(*entry.key());
                    entry.remove();
                    updated = true;
                } else if now
                    .duration_since(entry.get().last_contact)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    > Duration::from_secs(3)
                    && !entry.get().stale
                {
                    tracing::debug!(remote_socket = %entry.key(), remote_address = %entry.get().address, "marking peer as stale");
                    entry.get_mut().stale = true;
                    updated = true;
                }
            }
        }
        if updated {
            self.update_peer_list_message_cache(rt, Some(now));
        }
    }

    fn update_peer_list(
        &mut self,
        rt: &mut R,
        remote_peer_list: &[SocketAddr],
        now: Option<<R::Clock as Clock>::TimePoint>,
    ) {
    }

    fn build_peer_list<'a>(
        now: <R::Clock as Clock>::TimePoint,
        peers: impl Iterator<Item = &'a Peer<R>>,
    ) -> Vec<SocketAddr> {
        vec![]
    }

    fn update_peer_list_message_cache(
        &mut self,
        rt: &mut R,
        now: Option<<R::Clock as Clock>::TimePoint>,
    ) {
    }

    fn peers_try_send(
        rt: &mut R,
        now: Option<<R::Clock as Clock>::TimePoint>,
        peers: &mut HashMap<R::Socket, Peer<R>>,
        buf: &[u8],
    ) {
    }

    fn peer_try_send(
        rt: &mut R,
        now: <R::Clock as Clock>::TimePoint,
        socket: &R::Socket,
        peer: &mut Peer<R>,
        buf: &[u8],
    ) {
    }

    fn add_start_peers(
        &mut self,
        _: &mut R,
        _: &Vec<SocketAddr>,
        _: Option<<R::Clock as Clock>::TimePoint>,
    ) {
    }
}

impl<R> Protocol<R> for SimpleProto<R>
where
    R: Runtime,
{
    type Ctl = SimpleProtoCtl;

    fn run(&mut self, rt: &mut R, _iov: &[Event<R, Self::Ctl>]) -> ProtocolEvent<R> {
        todo!()
    }

    /*

    fn ctl(&mut self, rt: &mut R, _: Self::Ctl) -> Result<(), ()> {
        if let Some(peers) = self.start_peers.take() {
            self.add_start_peers(rt, &peers, None);
        }
        Ok(())
    }

    fn receive(
        &mut self,
        rt: &mut R,
        socket: <R as Runtime>::Socket,
        buf: &[u8],
    ) -> ProtocolEvent<R> {
        if let Some(peer) = self.peers.get_mut(&socket) {
            peer.last_contact = rt.get_default_clock().now();
            if peer.stale {
                peer.stale = false;
                tracing::debug!(remote_socket = %socket, remote_address = %peer.address, "peer not longer marked stale");
                self.update_peer_list_message_cache(rt, None);
            }
        }
        match bincode::deserialize::<Vec<SocketAddr>>(buf) {
            Err(error) => {
                tracing::warn!(?error, remote_socket = %socket, "received corrupt message")
            }
            Ok(list) => self.update_peer_list(rt, &list, None),
        }
        self.cleanup_dead_peers(rt, None);
        ProtocolEvent::asap(&rt.get_default_clock())
    }

    fn writeable(&mut self, rt: &mut R, socket: <R as Runtime>::Socket) -> ProtocolEvent<R> {
        let now = rt.get_default_clock().now();
        if let Some(peer) = self.peers.get_mut(&socket) {
            Self::peer_try_send(rt, now, &socket, peer, &self.peer_list_message)
        }
        self.cleanup_dead_peers(rt, Some(now));
        ProtocolEvent::asap(&rt.get_default_clock())
    }

    fn wake(&mut self, rt: &mut R) -> ProtocolEvent<R> {
        Self::peers_try_send(rt, None, &mut self.peers, &self.peer_list_message);
        self.cleanup_dead_peers(rt, None);
        ProtocolEvent::asap(&rt.get_default_clock())
    }

    */
}
