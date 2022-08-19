use super::handshake;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Error {
    InvalidLength { expected: usize, actual: usize },
    Handshake(handshake::error::Error),
    UnknownContentType(u8),
    UnknownVersion([u8; 2]),
    TooSmall(usize),
    CCS(super::ccs::error::Error),
    App(super::app::error::Error),
}

pub fn unknown_version(v: [u8; 2]) -> Error {
    Error::UnknownVersion(v)
}

pub fn too_small(s: usize) -> Error {
    Error::TooSmall(s)
}

pub fn unexpected_length(expected: usize, actual: usize) -> Error {
    Error::InvalidLength { expected, actual }
}

pub fn unknown_content_type(t: u8) -> Error {
    Error::UnknownContentType(t)
}

impl From<handshake::error::Error> for Error {
    fn from(e: handshake::error::Error) -> Self {
        Self::Handshake(e)
    }
}
