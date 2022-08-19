use core::convert::TryFrom;

pub mod rist;

/// A RTP Header Extension
pub trait ReadExt<'a>: TryFrom<&'a [u8]> {}
pub trait WriteExt {}
