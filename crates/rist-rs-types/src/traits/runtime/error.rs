use core::fmt::{Debug, Display};

#[cfg(feature = "std")]
use std::boxed::Box;

pub enum Error {
    WouldBlock,
    NoBuffers,
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
            _ if value.raw_os_error().unwrap_or(0) == 55 => Self::NoBuffers,
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
            Self::NoBuffers => write!(f, "NoBuffers"),
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
            Self::NoBuffers => Self::NoBuffers,
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
