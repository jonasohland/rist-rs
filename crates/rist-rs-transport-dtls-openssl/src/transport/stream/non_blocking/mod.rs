use crate::transport::stream::Config;
use crate::transport::stream::SslContextProvider;
use crate::util;

use rist_rs_buffers::stream::message::MessageStreamPeerAddress;
use rist_rs_buffers::stream::message::NonBlockingMessageStream;
use rist_rs_buffers::stream::message::NonBlockingMessageStreamAcceptor;
use rist_rs_buffers::stream::message::NonBlockingMessageStreamConnector;
use rist_rs_core::traits::io::ReceiveFromNonBlocking;
use rist_rs_core::traits::io::ReceiveNonBlocking;
use rist_rs_core::traits::io::SendNonBlocking;
use rist_rs_core::traits::io::SendToNonBlocking;

use std::borrow::Borrow;
use std::collections::LinkedList;
use std::fmt::Debug;
use std::io;
use std::io::Read;
use std::io::Write;
use std::time::Duration;
use std::time::SystemTime;

use openssl::error::ErrorStack as SslErrorStack;
use openssl::ssl;

/// Wrapper for NonBlockingMessage stream for which Read/Write can be implemented
pub struct StreamWrapper<A>
where
    A: MessageStreamPeerAddress,
{
    inner: NonBlockingMessageStream<A>,
}

impl<A> StreamWrapper<A>
where
    A: MessageStreamPeerAddress,
{
    /// Wrap the non blocking message stream
    fn new(inner: NonBlockingMessageStream<A>) -> Self {
        Self { inner }
    }
}

impl<A> Read for StreamWrapper<A>
where
    A: MessageStreamPeerAddress,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        util::non_blocking_stream_to_io_result(self.inner.try_recv(buf))
    }
}

impl<A> Write for StreamWrapper<A>
where
    A: MessageStreamPeerAddress,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        util::non_blocking_stream_to_io_result(self.inner.try_send(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// States of the DTLS shutdown process
#[derive(Clone, Copy, Debug)]
pub enum ShutdownState {
    /// The stream is active and not shut-down
    Active,

    /// Shutdown message is sent or an initial shutdown message was received
    ShuttingDown,

    /// Shutdown complete
    Shutdown,
}

impl From<ssl::ShutdownResult> for ShutdownState {
    fn from(res: ssl::ShutdownResult) -> Self {
        match res {
            ssl::ShutdownResult::Sent => ShutdownState::ShuttingDown,
            ssl::ShutdownResult::Received => ShutdownState::Shutdown,
        }
    }
}

impl ShutdownState {
    pub fn is_shutdown(&self) -> bool {
        matches!(self, ShutdownState::Shutdown)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, ShutdownState::Active)
    }
}

/// A non-blocking DTLS stream
pub struct DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    inner: ssl::SslStream<StreamWrapper<A>>,
}

impl<A> DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    /// Creates a new, active Dtls stream
    fn new(inner: ssl::SslStream<StreamWrapper<A>>) -> Self {
        Self { inner }
    }

    /// Convert a Ssl error to a non-blocking operation style result
    fn ssl_err_to_non_blocking_result<R>(
        res: Result<R, openssl::ssl::Error>,
    ) -> Option<Result<R, openssl::ssl::Error>> {
        match res {
            Err(e)
                if e.code() == ssl::ErrorCode::WANT_READ
                    || e.code() == ssl::ErrorCode::WANT_WRITE =>
            {
                None
            }
            Err(e) => Some(Err(e)),
            Ok(res) => Some(Ok(res)),
        }
    }

    /// Get the remote peer address
    pub fn peer_address(&self) -> A {
        self.inner.get_ref().inner.peer_address()
    }

    pub fn shutdown_state(&mut self) -> ShutdownState {
        match self.inner.get_shutdown() {
            k if k.contains(ssl::ShutdownState::RECEIVED | ssl::ShutdownState::SENT) => {
                ShutdownState::Shutdown
            }
            k if k.intersects(ssl::ShutdownState::RECEIVED | ssl::ShutdownState::SENT) => {
                ShutdownState::ShuttingDown
            }
            _ => ShutdownState::Active,
        }
    }

    /// Shut down the stream. This is a non-blocking operation that must be called multiple times
    /// until it returns a result
    pub fn shutdown(&mut self) -> Option<Result<(), DtlsStreamError>> {
        if self.shutdown_state().is_active() {
            tracing::debug!(peer = ?self.peer_address(), "shutting down dtls stream");
        }
        match self.shutdown_state() {
            ShutdownState::Active | ShutdownState::ShuttingDown => {
                match Self::ssl_err_to_non_blocking_result(self.inner.shutdown()) {
                    Some(Ok(_)) => {
                        if self.shutdown_state().is_shutdown() {
                            tracing::debug!(peer = ?self.peer_address(), "stream shutdown complete");
                            Some(Ok(()))
                        } else {
                            None
                        }
                    }
                    Some(Err(e)) => Some(Err(e.into())),
                    None => None,
                }
            }
            ShutdownState::Shutdown => Some(Ok(())),
        }
    }

    /// Transform a ssl result to a non-blocking style result with a DtlsStreamError
    fn transform_ssl_result<I>(res: Result<I, ssl::Error>) -> Option<Result<I, DtlsStreamError>> {
        match res {
            Err(e)
                if matches!(e.code(), openssl::ssl::ErrorCode::WANT_READ)
                    || matches!(e.code(), openssl::ssl::ErrorCode::WANT_WRITE) =>
            {
                None
            }
            Err(e) => Some(Err(DtlsStreamError::Ssl(e))),
            Ok(r) => Some(Ok(r)),
        }
    }
}

/// Drop implemented to last-second close a open ssl stream
impl<A> Drop for DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    fn drop(&mut self) {
        if !self.shutdown_state().is_shutdown() {
            tracing::warn!(
                "open dtls stream dropped, try sending shutdown message once and abort stream"
            );
            self.inner.shutdown().ok();
        }
    }
}

impl<A> SendNonBlocking for DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    type Error = DtlsStreamError;

    fn try_send(&mut self, buf: &[u8]) -> Option<Result<usize, Self::Error>> {
        Self::transform_ssl_result(self.inner.ssl_write(buf))
    }
}

impl<A> ReceiveFromNonBlocking for DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    type Error = DtlsStreamError;
    type Address = A;

    fn try_recv_from(
        &mut self,
        buf: &mut [u8],
    ) -> Option<Result<(usize, Self::Address), Self::Error>> {
        Self::transform_ssl_result(
            self.inner
                .ssl_read(buf)
                .map(|len| (len, self.peer_address())),
        )
    }
}

impl<A> ReceiveNonBlocking for DtlsStream<A>
where
    A: MessageStreamPeerAddress,
{
    type Error = DtlsStreamError;

    fn try_recv(&mut self, buf: &mut [u8]) -> Option<Result<usize, Self::Error>> {
        Self::transform_ssl_result(self.inner.ssl_read(buf))
    }
}

/// Error returned by a DTLS stream
#[derive(Debug)]
pub enum DtlsStreamError {
    Library(openssl::error::ErrorStack),
    Ssl(openssl::ssl::Error),
    Io(io::Error),
}

impl From<openssl::ssl::Error> for DtlsStreamError {
    fn from(e: openssl::ssl::Error) -> Self {
        match e.into_io_error() {
            Ok(ioe) => Self::Io(ioe),
            Err(e) => Self::Ssl(e),
        }
    }
}

/// A stream candidate used to keep state inside the stream acceptor
enum DtlsStreamCandidate<A>
where
    A: MessageStreamPeerAddress,
{
    Candidate(ssl::MidHandshakeSslStream<StreamWrapper<A>>),
    Ready(DtlsStream<A>),
}

/// Result of a accept operation that might still be in progress
enum DtlsStreamCandidateError<A>
where
    A: MessageStreamPeerAddress,
{
    InProgress(DtlsStreamCandidate<A>),
    Error(SslErrorStack, A),
    Failed(openssl::ssl::Error, A),
}

enum Direction {
    Accept,
    Connect,
}

impl<A> DtlsStreamCandidate<A>
where
    A: MessageStreamPeerAddress,
{
    fn candidate(state: ssl::MidHandshakeSslStream<StreamWrapper<A>>) -> Self {
        Self::Candidate(state)
    }

    fn ready(stream: DtlsStream<A>) -> Self {
        Self::Ready(stream)
    }

    /// Get the remote peer address
    fn peer_address(&self) -> A {
        match self {
            DtlsStreamCandidate::Candidate(state) => state.get_ref().inner.peer_address(),
            DtlsStreamCandidate::Ready(s) => s.inner.get_ref().inner.peer_address(),
        }
    }

    /// Try creating a new stream candidate from a message stream and a reference to a SslContext
    fn try_new(
        stream: NonBlockingMessageStream<A>,
        context: impl Borrow<ssl::SslContext>,
        mtu: usize,
        direction: Direction,
    ) -> Result<Self, DtlsStreamError> {
        let ssl = ssl::Ssl::new(context.borrow())
            .and_then(|mut ssl| {
                ssl.set_mtu(mtu as u32)?;
                Ok(ssl)
            })
            .map_err(DtlsStreamError::Library)?;
        match match direction {
            Direction::Accept => ssl.accept(StreamWrapper::new(stream)),
            Direction::Connect => ssl.connect(StreamWrapper::new(stream)),
        } {
            Err(ssl::HandshakeError::WouldBlock(mid_handshake_stream)) => {
                Ok(DtlsStreamCandidate::Candidate(mid_handshake_stream))
            }
            Err(ssl::HandshakeError::Failure(e)) => Err(DtlsStreamError::Ssl(e.into_error())),
            Err(ssl::HandshakeError::SetupFailure(e)) => Err(DtlsStreamError::Library(e)),
            Ok(stream) => Ok(DtlsStreamCandidate::ready(DtlsStream::new(stream))),
        }
    }

    /// Try accepting the stream by processing all pending events. This function must be called repeatedly until it
    /// returns a result
    fn try_handshake(self) -> Result<DtlsStream<A>, DtlsStreamCandidateError<A>> {
        let peer = self.peer_address();
        match self {
            Self::Candidate(state) => match state.handshake() {
                Ok(s) => Ok(DtlsStream::new(s)),
                Err(ssl::HandshakeError::WouldBlock(returned_stream)) => {
                    Err(DtlsStreamCandidateError::InProgress(
                        DtlsStreamCandidate::candidate(returned_stream),
                    ))
                }
                Err(ssl::HandshakeError::SetupFailure(err)) => {
                    Err(DtlsStreamCandidateError::Error(err, peer))
                }
                Err(ssl::HandshakeError::Failure(failed)) => {
                    Err(DtlsStreamCandidateError::Failed(failed.into_error(), peer))
                }
            },
            DtlsStreamCandidate::Ready(stream) => Ok(stream),
        }
    }
}

struct DtlsStreamCandidateStore<Address>
where
    Address: MessageStreamPeerAddress,
{
    candidates: Option<LinkedList<(DtlsStreamCandidate<Address>, SystemTime)>>,
}

impl<A> DtlsStreamCandidateStore<A>
where
    A: MessageStreamPeerAddress,
{
    fn new() -> Self {
        Self { candidates: None }
    }

    fn new_candidate(
        &mut self,
        context: impl Borrow<ssl::SslContext>,
        stream: NonBlockingMessageStream<A>,
        direction: Direction,
    ) -> Option<io::Error> {
        let now = SystemTime::now();
        let peer = stream.peer_address();
        match DtlsStreamCandidate::try_new(stream, context, 2500, direction) {
            Ok(candidate) => {
                tracing::debug!(?peer, "new dtls stream candidate");
                self.candidates
                    .get_or_insert_with(Default::default)
                    .push_back((candidate, now));
                None
            }
            Err(DtlsStreamError::Io(ioe)) => Some(ioe),
            Err(DtlsStreamError::Library(error)) => {
                tracing::warn!(?peer, ?error, "could not create new dtls stream candidate");
                None
            }
            Err(DtlsStreamError::Ssl(error)) => {
                tracing::info!(?peer, ?error, "dtls handshake failed");
                None
            }
        }
    }

    fn run(&mut self) -> Option<Vec<DtlsStream<A>>> {
        let now = SystemTime::now();
        let mut output: Option<Vec<_>> = None;
        self.candidates = self.candidates.take().map(|candidate_list| {
                    // map the whole list to a new list with non-filtered candidates
                    candidate_list
                        .into_iter()
                        .filter_map(|candidate| {
                            // Filter timed-out candidates
                            if now.duration_since(candidate.1).unwrap_or_else(|_| Duration::from_secs(0)) > Duration::from_secs(10) {
                                tracing::debug!(peer = ?candidate.0.peer_address() ,"dtls handshake timed out");
                                None
                            }
                            else {
                                match candidate.0.try_handshake() {
                                    Ok(stream) => {
                                        tracing::debug!(peer = ?stream.peer_address(), "dtls connection established");
                                        output.get_or_insert_with(Vec::new).push(stream);
                                        None
                                    }
                                    Err(DtlsStreamCandidateError::InProgress(stream)) => Some((stream, candidate.1)),
                                    Err(DtlsStreamCandidateError::Error(error, peer)) => {
                                        tracing::error!(
                                            ?peer, ?error, msg = %error,
                                            "dtls handshake failed because of an unexpected openssl error"
                                        );
                                        None
                                    }
                                    Err(DtlsStreamCandidateError::Failed(error, peer)) => {
                                        tracing::info!(?peer, ?error, msg = %error, "dtls handshake failed");
                                        None
                                    }
                                }
                            }
                        })
                        .collect()
                });
        output
    }
}

/// Error returned by the DTLS stream acceptor
#[derive(Debug)]
pub enum DtlsStreamAcceptError<EA, EC>
where
    EA: Debug,
    EC: Debug,
{
    /// The underlying stream acceptor returned an error
    Accept(EA),

    /// The context provider returned an error
    Context(EC),

    /// An IO error ocurred
    Io(io::Error),
}

/// Accepts DTLS streams in a non-blocking fashion. Built on NonBlockingMessageStream.
pub struct DtlsStreamAcceptor<Stream, Address, ContextProvider, AcceptErr, ContextErr>
where
    Address: MessageStreamPeerAddress,
    Stream: ReceiveFromNonBlocking<Error = AcceptErr, Address = Address>
        + SendToNonBlocking<Error = AcceptErr, Address = Address>,
    ContextProvider: SslContextProvider<Error = ContextErr>,
{
    config: Config<ContextProvider>,
    acceptor: NonBlockingMessageStreamAcceptor<Stream, Address, AcceptErr>,
    candidates: DtlsStreamCandidateStore<Address>,
}

impl<Stream, Address, ContextProvider, ContextError, AcceptError>
    DtlsStreamAcceptor<Stream, Address, ContextProvider, AcceptError, ContextError>
where
    AcceptError: Debug,
    ContextError: Debug,
    Address: MessageStreamPeerAddress,
    Stream: ReceiveFromNonBlocking<Error = AcceptError, Address = Address>
        + SendToNonBlocking<Error = AcceptError, Address = Address>,
    ContextProvider: SslContextProvider<Error = ContextError>,
{
    /// Create a new DTLS stream acceptor
    pub fn new(io: Stream, mtu: usize, config: Config<ContextProvider>) -> Self {
        Self {
            config,
            acceptor: NonBlockingMessageStreamAcceptor::new(io, mtu),
            candidates: DtlsStreamCandidateStore::new(),
        }
    }

    /// Accept new DTLS streams. This function will never block must be called repeatedly
    #[allow(clippy::type_complexity)]
    pub fn try_accept(
        &mut self,
    ) -> Option<Result<Vec<DtlsStream<Address>>, DtlsStreamAcceptError<AcceptError, ContextError>>>
    {
        match loop {
            match self.acceptor.accept().map(|s| match s {
                Err(e) => Err(DtlsStreamAcceptError::Accept(e)),
                Ok(stream) => match self.config.context_builder.context() {
                    Err(ec) => Err(DtlsStreamAcceptError::Context(ec)),
                    Ok(context) => {
                        match self
                            .candidates
                            .new_candidate(context, stream, Direction::Accept)
                        {
                            Some(e) => Err(DtlsStreamAcceptError::Io(e)),
                            None => Ok(()),
                        }
                    }
                },
            }) {
                Some(Ok(_)) => continue,
                Some(Err(err)) => break Some(err),
                None => break None,
            }
        } {
            // An error was returned from the accept() loop, propagate to user
            Some(err) => Some(Err(err)),

            // No error was returned from the accept() loop, start processing candidate events
            None => self.candidates.run().map(Ok),
        }
    }
}

pub struct DtlsStreamConnector<Stream, Address, ContextProvider, ConnectError, ContextError>
where
    Address: MessageStreamPeerAddress,
    Stream: ReceiveFromNonBlocking<Error = ConnectError, Address = Address>
        + SendToNonBlocking<Error = ConnectError, Address = Address>,
    ContextProvider: SslContextProvider<Error = ContextError>,
{
    config: Config<ContextProvider>,
    connector: NonBlockingMessageStreamConnector<Stream, Address, ConnectError>,
    candidates: DtlsStreamCandidateStore<Address>,
}

/// Error returned by the DTLS stream connector
#[derive(Debug)]
pub enum DtlsStreamConnectError<EN, EC>
where
    EN: Debug,
    EC: Debug,
{
    /// The underlying stream connector returned an error
    Connect(EN),

    /// The context provider returned an error
    Context(EC),

    /// An IO error ocurred
    Io(io::Error),
}

impl<Stream, Address, ContextProvider, ContextError, ConnectError>
    DtlsStreamConnector<Stream, Address, ContextProvider, ConnectError, ContextError>
where
    ConnectError: Debug,
    ContextError: Debug,
    Address: MessageStreamPeerAddress,
    Stream: ReceiveFromNonBlocking<Error = ConnectError, Address = Address>
        + SendToNonBlocking<Error = ConnectError, Address = Address>,
    ContextProvider: SslContextProvider<Error = ContextError>,
{
    /// Create a new DTLS stream acceptor
    pub fn new(io: Stream, mtu: usize, config: Config<ContextProvider>) -> Self {
        Self {
            config,
            connector: NonBlockingMessageStreamConnector::new(io, mtu),
            candidates: DtlsStreamCandidateStore::new(),
        }
    }

    pub fn add(
        &mut self,
        address: impl Borrow<Address>,
    ) -> Option<Result<(), DtlsStreamConnectError<ConnectError, ContextError>>> {
        self.config
            .context_builder
            .context()
            .map_err(DtlsStreamConnectError::Context)
            .and_then(|ctx| {
                match self
                    .candidates
                    .new_candidate(ctx, self.connector.connect(address), Direction::Connect)
                    .map(DtlsStreamConnectError::Io)
                {
                    Some(e) => Err(e),
                    None => Ok(()),
                }
            })
            .err()
            .map(Err)
    }

    #[allow(clippy::type_complexity)]
    pub fn try_connect(
        &mut self,
    ) -> Option<Result<Vec<DtlsStream<Address>>, DtlsStreamConnectError<ConnectError, ContextError>>>
    {
        match self.connector.run() {
            Some(Err(e)) => Some(Err(DtlsStreamConnectError::Connect(e))),
            _ => self.candidates.run().map(Ok),
        }
    }
}
