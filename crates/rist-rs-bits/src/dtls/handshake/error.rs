#[derive(Debug)]
pub enum Error {
    InvalidLength { expected: usize, actual: usize },
    TooSmall(usize),
    UnknownBodyType(u8),
}

pub fn invalid_length(expected: usize, actual: usize) -> Error {
    Error::InvalidLength { expected, actual }
}

pub fn too_small(l: usize) -> Error {
    Error::TooSmall(l)
}

pub fn unknown_body_type(t: u8) -> Error {
    Error::UnknownBodyType(t)
}
