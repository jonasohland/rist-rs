#![allow(unused)]
use core::{convert::TryFrom, fmt::Display};

use super::util;
mod error;

/// Read the version number of an ip packet.
/// Returns the Ip version fields value or an error if the slice is empty
pub fn read_ip_version(data: &[u8]) -> Result<u8, error::Error> {
    if data.is_empty() {
        Err(error::Error::General(error::general::empty()))
    } else {
        Ok((data[0] & 0xf0) >> 4)
    }
}

enum IpPacketView<'a> {
    V4(v4::Ipv4PacketView<'a>),
}

impl<'a> TryFrom<&'a [u8]> for IpPacketView<'a> {
    type Error = error::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        match read_ip_version(data)? {
            4 => Ok(IpPacketView::V4(v4::Ipv4PacketView::try_from(data)?)),
            v => Err(error::Error::General(
                error::general::ip_version_not_implemented(v),
            )),
        }
    }
}

impl<'a> IpPacketView<'a> {
    fn source_addr(&self) -> rist_rs_core::net::IpAddr {
        match self {
            Self::V4(a) => a.source_addr().into(),
        }
    }

    fn dest_addr(&self) -> rist_rs_core::net::IpAddr {
        match self {
            Self::V4(a) => a.dest_addr().into(),
        }
    }

    fn v4(&self) -> Option<v4::Ipv4PacketView<'a>> {
        match self {
            Self::V4(v) => Some(*v),
            _ => None,
        }
    }
}

impl<'a> Display for IpPacketView<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IpPacketView::V4(v4) => v4.fmt(f),
        }
    }
}

pub mod v4;
