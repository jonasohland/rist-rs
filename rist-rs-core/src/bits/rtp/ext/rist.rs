use core::convert::TryFrom;

use super::super::RTPView;

#[derive(Debug)]
pub enum ErrorKind {
    InvalidLength,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

pub struct Extension<'a> {
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for Extension<'a> {
    type Error = Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self { data })
    }
}

impl<'a> super::ReadExt<'a> for Extension<'a> {}

impl<'a> Extension<'a> {
    fn has_extended_sequence_number(&self) -> bool {
        todo!()
    }

    fn extended_sequence_number(&self, packet: &RTPView) -> Option<u32> {
        todo!()
    }
}
