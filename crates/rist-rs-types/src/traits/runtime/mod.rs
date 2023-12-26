use bitflags::bitflags;
use core::{
    fmt::{Debug, Display},
    hash::Hash,
};

use super::time::clock::Clock;

pub trait StdError: Debug {}

bitflags! {
    pub struct SocketFlags: u32 {
        const RecvFrom = 0x000001;
    }
}

pub enum Error {
    WouldBlock,
    AddrInUse,
    AlreadyExists,
    InvalidInput,
    Str(&'static str),
    Any(Box<dyn Display>),

    #[cfg(feature = "std")]
    Boxed(Box<dyn std::error::Error>),
}

#[cfg(feature = "std")]
impl From<Box<dyn std::error::Error>> for Error {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Boxed(value)
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::WouldBlock => Self::WouldBlock,
            std::io::ErrorKind::AddrInUse => Self::AddrInUse,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::InvalidInput => Self::InvalidInput,
            _ => Self::Any(Box::new(value)),
        }
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Self::Str(value)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WouldBlock => write!(f, "WouldBlock"),
            Self::AddrInUse => write!(f, "AddrInUse"),
            Self::AlreadyExists => write!(f, "AlreadyExists"),
            Self::InvalidInput => write!(f, "InvalidInput"),
            Self::Str(s) => write!(f, "{}", s),
            Self::Any(arg0) => f.debug_tuple("Any").field(&arg0.to_string()).finish(),

            #[cfg(feature = "std")]
            Self::Boxed(err) => f.debug_tuple("Boxed").field(&err.to_string()).finish(),
        }
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::WouldBlock => Self::WouldBlock,
            Self::AddrInUse => Self::AddrInUse,
            Self::AlreadyExists => Self::AlreadyExists,
            Self::InvalidInput => Self::InvalidInput,
            Self::Str(arg0) => Self::Str(arg0),
            Self::Any(arg0) => Self::Any(Box::new(arg0.to_string())),

            #[cfg(feature = "std")]
            Self::Boxed(arg0) => Self::Any(Box::new(arg0.to_string())),
        }
    }
}

pub trait SocketAddr: Debug + Display + Clone + Copy + PartialEq + Eq + Hash + Send
where
    Self: From<std::net::SocketAddr>,
{
    fn network_address(&self) -> Option<&std::net::SocketAddr>;
}

pub trait Socket: Debug + Display + Clone + Copy + PartialEq + Eq + Hash + Send {}

pub trait Runtime: Send + 'static {
    type Clock: Clock;

    type SocketAddr: SocketAddr;

    type Socket: Socket;

    fn get_clock(&mut self, id: Option<&str>) -> Self::Clock;

    fn get_default_clock(&mut self) -> Self::Clock {
        self.get_clock(None)
    }

    fn get_remote_address(&self, remote: Self::Socket) -> Result<Self::SocketAddr, Error>;

    fn bind(
        &mut self,
        address: Self::SocketAddr,
        flags: SocketFlags,
    ) -> Result<Self::Socket, Error>;

    fn connect(&mut self, socket: Self::Socket, address: Self::SocketAddr) -> Result<(), Error>;

    fn send(&mut self, socket: Self::Socket, buf: &[u8]) -> Result<(), Error>;

    fn send_to(
        &mut self,
        socket: Self::Socket,
        buf: &[u8],
        address: Self::SocketAddr,
    ) -> Result<(), Error>;

    fn recv(&mut self, socket: Self::Socket, buf: &mut [u8]) -> Result<usize, Error>;

    fn recv_from(
        &mut self,
        socket: Self::Socket,
        buf: &mut [u8],
    ) -> Result<(usize, Self::SocketAddr), Error>;

    fn close(&mut self, socket: Self::Socket);
}
