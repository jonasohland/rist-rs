#[derive(Debug)]
pub enum Kind {
    Other(&'static str),
}

#[derive(Debug)]
pub struct Error {
    k: Kind,
}

pub fn other(s: &'static str) -> Error {
    Error { k: Kind::Other(s) }
}
