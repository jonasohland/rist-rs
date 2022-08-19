/// General errors returned by the ip packet parsing/writing facilities
pub mod general {

    #[derive(Debug, Clone, Copy)]
    pub enum ErrorKind {

        /// The given slice of data was empty
        Empty,

        /// The ip version indicated by the ip version field is not supported by this implementation
        IpVersionNotImplemented(u8),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Error {
        kind: ErrorKind,
    }

    impl Error {

        /// Get the kind of error for this error object
        pub fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    /// Make an error to indicate that the given ip version is not implemented
    pub fn ip_version_not_implemented(v: u8) -> Error {
        Error {
            kind: ErrorKind::IpVersionNotImplemented(v),
        }
    }

    /// Make an error to indicate that the given slice of data is empty
    pub fn empty() -> Error {
        Error {
            kind: ErrorKind::Empty,
        }
    }
}

#[derive(Debug)]
pub enum Error {

    /// General error
    General(general::Error),

    /// Ipv4 related error
    V4(super::v4::error::Error),

    /// Ipv6 related error
    V6(),
}

impl From<super::v4::error::Error> for Error {
    fn from(err: super::v4::error::Error) -> Self {
        Error::V4(err)
    }
}

impl Error {
    /// True if the error is a Ipv4-related error
    fn is_v4(&self) -> bool {
        matches!(self, Error::V4(_))
    }

    /// True is the error is Ipv6-related error
    fn is_v6(&self) -> bool {
        matches!(self, Error::V6())
    }

    /// True if the error is a general error
    fn is_general(&self) -> bool {
        matches!(self, Error::General((_)))
    }
}
