use core::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    /// Invalid value for the ip packet version
    InvalidVersion(u8),

    /// Wrong value for the ip packet version
    WrongVersion(u8, u8),

    /// Not enough data was supplied to read the value of a specific field
    NotEnoughData {
        need: usize,
        got: usize,
        field: &'static &'static str,
    },

    /// The value of the IHL field is out of the legal bounds
    HeaderTooLong,
}

#[derive(Debug, Clone, Copy)]
pub struct Error {
    kind: ErrorKind,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            ErrorKind::InvalidVersion(v) => write!(f, "Invalid Ip protocol version '{}'", v),
            ErrorKind::WrongVersion(expected, got) => {
                write!(f, "Wrong ip version, expected: {}, got: {}", expected, got)
            }
            ErrorKind::NotEnoughData { need, got, field } => {
                write!(f, "Not enough data to read value(s) from field [{}], need at least {} bytes, got {} bytes", **field, need, got)
            }
            ErrorKind::HeaderTooLong => {
                write!(f, "Header reported as longer than total packet size")
            }
        }
    }
}

impl Error {
    /// Create a new error from a given ErrorKind
    fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    /// Extract the error kind
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

/// Make an error that indicates a wrong value of the ip version field
pub(super) fn wrong_version(expected: u8, got: u8) -> Error {
    Error::new(ErrorKind::WrongVersion(expected, got))
}

/// Make an error that indicates that not enough data was supplied to read the value of a part of the IP packet
pub(super) fn not_enough_data(need: usize, got: usize, field: &'static &'static str) -> Error {
    Error::new(ErrorKind::NotEnoughData { need, got, field })
}

/// Make an error that indicates that the IHL field has a value that is out of the legal bounds for this packet
pub(super) fn header_to_long() -> Error {
    Error::new(ErrorKind::HeaderTooLong)
}
