use super::MessageStreamPeerAddress;
use crate::channel::mpmc::{channel, Receiver, Sender, TryRecvError, TrySendError};

use rist_rs_core::collections::static_vec::StaticVec;
use rist_rs_core::traits::io::{ReceiveFromNonBlocking, ReceiveNonBlocking};
use rist_rs_core::traits::io::{SendNonBlocking, SendToNonBlocking};

use core::fmt::Debug;

use hashbrown::HashMap;

use tracing::debug;

struct DuplexChannel<T, R> {
    tx: Sender<T>,
    rx: Receiver<R>,
}

impl<T, R> DuplexChannel<T, R> {
    fn is_disconnected(&self) -> bool {
        self.rx.is_disconnected() || self.tx.is_disconnected()
    }

    fn try_send(&mut self, data: T) -> Result<(), TrySendError<T>> {
        self.tx.try_send(data)
    }

    fn try_recv(&mut self) -> Result<R, TryRecvError> {
        self.rx.try_receive()
    }
}

fn duplex_channel<T, R>(cap: usize) -> (DuplexChannel<T, R>, DuplexChannel<R, T>) {
    let (tx1, rx1) = channel(cap);
    let (tx2, rx2) = channel(cap);
    (
        DuplexChannel { tx: tx1, rx: rx2 },
        DuplexChannel { tx: tx2, rx: rx1 },
    )
}

struct NonBlockingMessageStreamChannel {
    channel: DuplexChannel<StaticVec<u8>, StaticVec<u8>>,
    buffered: Option<StaticVec<u8>>,
    dropped: u64,
}

#[derive(Debug)]
enum NonBlockingMessageStreamRxError<T> {
    Disconnected(T),
    Dropped,
}

#[derive(Debug)]
enum NonBlockingMessageStreamTxError {
    Disconnected,
    Empty,
}

impl NonBlockingMessageStreamChannel {
    fn new(
        channel: DuplexChannel<StaticVec<u8>, StaticVec<u8>>,
    ) -> NonBlockingMessageStreamChannel {
        NonBlockingMessageStreamChannel {
            channel,
            buffered: None,
            dropped: 0,
        }
    }

    fn is_disconnected(&self) -> bool {
        self.channel.is_disconnected()
    }

    fn try_tx(&mut self) -> Result<StaticVec<u8>, NonBlockingMessageStreamTxError> {
        if let Some(data) = self.buffered.take() {
            Ok(data)
        } else {
            match self.channel.try_recv() {
                Ok(data) => Ok(data),
                Err(TryRecvError::Empty) => Err(NonBlockingMessageStreamTxError::Empty),
                Err(TryRecvError::Disconnected) => {
                    Err(NonBlockingMessageStreamTxError::Disconnected)
                }
            }
        }
    }

    fn on_tx_failed(&mut self, data: StaticVec<u8>) {
        self.buffered = Some(data)
    }

    fn on_recv(
        &mut self,
        data: StaticVec<u8>,
    ) -> Result<(), NonBlockingMessageStreamRxError<StaticVec<u8>>> {
        match self.channel.try_send(data) {
            Ok(_) => Ok(()),
            Err(TrySendError::Disconnected(p)) => {
                Err(NonBlockingMessageStreamRxError::Disconnected(p))
            }
            Err(TrySendError::Full(_)) => {
                self.dropped += 1;
                Err(NonBlockingMessageStreamRxError::Dropped)
            }
        }
    }
}

pub struct NonBlockingMessageStreamAcceptor<S, A, E>
where
    E: Sized,
    A: MessageStreamPeerAddress,
    S: ReceiveFromNonBlocking<Error = E, Address = A> + SendToNonBlocking<Error = E, Address = A>,
{
    io: S,
    accept_call_cnt: usize,
    rx_buf: StaticVec<u8>,
    streams: HashMap<A, NonBlockingMessageStreamChannel>,
}

impl<S, A, E> NonBlockingMessageStreamAcceptor<S, A, E>
where
    E: Sized,
    A: MessageStreamPeerAddress,
    S: ReceiveFromNonBlocking<Error = E, Address = A> + SendToNonBlocking<Error = E, Address = A>,
{
    pub fn new(io: S, mtu: usize) -> Self {
        Self {
            io,
            accept_call_cnt: 0,
            rx_buf: StaticVec::new(mtu),
            streams: Default::default(),
        }
    }

    fn clean_dead_channels(&mut self) {
        self.streams = self
            .streams
            .drain()
            .filter(|(peer, s)| {
                if s.is_disconnected() {
                    debug!(?peer, "remove dead message stream");
                    false
                } else {
                    true
                }
            })
            .collect();
    }

    fn maintenance(&mut self) {
        self.accept_call_cnt += 1;
        if self.accept_call_cnt >= 1024 {
            self.accept_call_cnt = 0;
            self.clean_dead_channels();
        }
    }

    fn emplace_new_stream_with_data(
        &mut self,
        data: StaticVec<u8>,
        peer: &A,
    ) -> NonBlockingMessageStream<A> {
        debug!(?peer, "create new message stream");
        let (c1, c2) = duplex_channel(1024);
        let mut backend = NonBlockingMessageStreamChannel::new(c1);
        backend.on_recv(data).unwrap();
        if self.streams.insert(*peer, backend).is_some() {
            debug!(?peer, "replace message stream backend")
        }
        NonBlockingMessageStream::new(c2, *peer)
    }

    fn on_rx(&mut self, len: usize, addr: &A) -> Option<NonBlockingMessageStream<A>> {
        match self.streams.get_mut(addr) {
            Some(channel) => match channel.on_recv(StaticVec::from(self.rx_buf.split_at(len).0)) {
                Ok(_) => None,
                Err(NonBlockingMessageStreamRxError::Dropped) => None,
                Err(NonBlockingMessageStreamRxError::Disconnected(data)) => {
                    Some(self.emplace_new_stream_with_data(data, addr))
                }
            },
            None => {
                Some(self.emplace_new_stream_with_data(
                    StaticVec::from(self.rx_buf.split_at(len).0),
                    addr,
                ))
            }
        }
    }

    pub fn accept(&mut self) -> Option<Result<NonBlockingMessageStream<A>, E>> {
        self.maintenance();
        while let Some(r) = self.io.try_recv_from(&mut self.rx_buf) {
            match r {
                Ok((len, addr)) => {
                    if let Some(stream) = self.on_rx(len, &addr) {
                        return Some(Ok(stream));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
        for (addr, ch) in &mut self.streams {
            loop {
                match ch.try_tx() {
                    Ok(data) => match self.io.try_send_to(&data, *addr) {
                        None => {
                            ch.on_tx_failed(data);
                            break;
                        }
                        Some(Err(err)) => {
                            ch.on_tx_failed(data);
                            return Some(Err(err));
                        }
                        Some(Ok(_)) => {
                            continue;
                        }
                    },
                    Err(NonBlockingMessageStreamTxError::Disconnected) => {
                        // ignore, stream will be cleaned up later
                        break;
                    }
                    Err(NonBlockingMessageStreamTxError::Empty) => {
                        break;
                    }
                }
            }
        }
        None
    }
}

pub enum NonBlockingMessageStreamError {
    Closed,
}

pub struct NonBlockingMessageStream<A>
where
    A: MessageStreamPeerAddress,
{
    channel: DuplexChannel<StaticVec<u8>, StaticVec<u8>>,
    peer_address: A,
}

impl<A> NonBlockingMessageStream<A>
where
    A: MessageStreamPeerAddress,
{
    fn new(
        channel: DuplexChannel<StaticVec<u8>, StaticVec<u8>>,
        peer_address: A,
    ) -> NonBlockingMessageStream<A> {
        Self {
            channel,
            peer_address,
        }
    }

    pub fn peer_address(&self) -> A {
        self.peer_address
    }
}

/*

impl io::Write for NonBlockingMessageStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.try_send(buf)
            .map(|r| r.map_err(|_| io::Error::from(io::ErrorKind::ConnectionAborted)))
            .unwrap_or_else(|| Err(io::Error::from(io::ErrorKind::WouldBlock)))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

impl io::Read for NonBlockingMessageStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.try_recv(buf)
            .map(|r| r.map_err(|_| io::Error::from(io::ErrorKind::ConnectionAborted)))
            .unwrap_or_else(|| Err(io::Error::from(io::ErrorKind::WouldBlock)))
    }
}

*/

impl<A> ReceiveNonBlocking for NonBlockingMessageStream<A>
where
    A: MessageStreamPeerAddress,
{
    type Error = NonBlockingMessageStreamError;

    fn try_recv(&mut self, buf: &mut [u8]) -> Option<Result<usize, Self::Error>> {
        match self.channel.try_recv() {
            Ok(data) => match data.len() {
                // provided buffer is large enough
                l if l < buf.len() => {
                    buf.split_at_mut(data.len()).0.copy_from_slice(&data);
                    Some(Ok(data.len()))
                }
                // provided buffer is too small
                l if l > buf.len() => {
                    buf.copy_from_slice(data.split_at(buf.len()).0);
                    Some(Ok(buf.len()))
                }
                // provided buffer fits exactly
                _ => {
                    buf.copy_from_slice(&data);
                    Some(Ok(data.len()))
                }
            },
            Err(TryRecvError::Disconnected) => Some(Err(NonBlockingMessageStreamError::Closed)),
            Err(TryRecvError::Empty) => None,
        }
    }
}

impl<A> SendNonBlocking for NonBlockingMessageStream<A>
where
    A: MessageStreamPeerAddress,
{
    type Error = NonBlockingMessageStreamError;

    fn try_send(&mut self, buf: &[u8]) -> Option<Result<usize, Self::Error>> {
        match self.channel.try_send(StaticVec::from(buf)) {
            Ok(_) => Some(Ok(buf.len())),
            Err(TrySendError::Disconnected(_)) => Some(Err(NonBlockingMessageStreamError::Closed)),
            Err(TrySendError::Full(_)) => None,
        }
    }
}
