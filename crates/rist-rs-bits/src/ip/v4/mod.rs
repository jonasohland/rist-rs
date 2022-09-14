/// Error types for Ipv4 packets
pub mod error;

/// Minimum length of the Ipv4 header
const IPV4_BASE_HEADER_LEN: usize = 20;

/// View over an immutable slice of data that can be interpreted as an Ipv4 packet.
#[derive(Debug, Clone, Copy)]
pub struct Ipv4PacketView<'a> {
    data: &'a [u8],
}

impl<'a> Ipv4PacketView<'a> {
    /// IHL (Internet Header Length)
    fn ihl(data: &[u8]) -> Result<usize, error::Error> {
        if data.is_empty() {
            Err(error::not_enough_data(1, 0, &"IPv4::InternetHeaderLength"))
        } else {
            Ok((data[0] & 0xf) as usize * core::mem::size_of::<u32>())
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Ipv4PacketView<'a> {
    type Error = super::error::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let header_len = Self::ihl(data)?;
        let pv = crate::ip::read_ip_version(data)?;
        // need at least 20 bytes of header
        if header_len < IPV4_BASE_HEADER_LEN {
            Err(
                error::not_enough_data(IPV4_BASE_HEADER_LEN, header_len, &"Ipv4Packet::Header")
                    .into(),
            )
        }
        // need protocol version 4
        else if pv != 4 {
            Err(error::wrong_version(4, pv).into())
        }
        // need at least [header_len] bytes of data
        else if data.len() < header_len {
            Err(error::not_enough_data(header_len, data.len(), &"Ipv4Packet::Header").into())
        }
        // ok, return the packet view
        else {
            Ok(Self { data })
        }
    }
}

impl<'a> Ipv4PacketView<'a> {
    /// Get the value of the header length field (IHL - Internet Header Length)
    pub fn header_len(&self) -> usize {
        Ipv4PacketView::ihl(self.data).expect(rist_rs_core::internal::INTERNAL_ERR_PRE_VALIDATED)
    }

    /// Get the value of the total length field
    pub fn total_len(&self) -> usize {
        u16::from_be_bytes([self.data[2], self.data[3]]) as usize
    }

    /// Returns a header length that is guaranteed to be valid.
    /// Returns an error if the header length is more than can be read from the slice
    fn valid_header_len(&self) -> Result<usize, error::Error> {
        if self.data.len() < self.header_len() {
            Err(error::not_enough_data(
                self.header_len(),
                self.data.len(),
                &"IPv4Packet::Header",
            ))
        } else {
            Ok(self.header_len())
        }
    }

    /// Returns a total packet length that is guaranteed to be valid.
    /// Returns an error if the total length is more than can be read from the slice
    fn valid_total_len(&self) -> Result<usize, error::Error> {
        if self.data.len() < self.total_len() {
            Err(error::not_enough_data(
                self.total_len(),
                self.data.len(),
                &"IPv4Packet",
            ))
        } else {
            Ok(self.total_len())
        }
    }

    /// Get the DSCP value from the header
    pub fn dscp(&self) -> u8 {
        (self.data[1] & 0xfc) >> 2
    }

    /// Get the ECN flag
    pub fn ecn(&self) -> u8 {
        self.data[1] & 0x3
    }

    /// Get the fragmentation identification value
    pub fn identification(&self) -> u16 {
        super::util::read_int!(self.data, u16, 4)
    }

    /// Get the DF (Don't Fragment) flag value
    pub fn df(&self) -> bool {
        (self.data[6] & 0x40) != 0
    }

    /// Get the MF (More Fragments) flag value
    pub fn mf(&self) -> bool {
        (self.data[6] & 0x20) != 0
    }

    /// Get the offset field value (Offset of this fragment in the fragmentation group)
    pub fn offset(&self) -> usize {
        u16::from_be_bytes([self.data[6] & 0x1f, self.data[7]]) as usize * 8
    }

    /// Check if the packet is part of a fragmented ip packet. Returns true if either the MF flag is set or the offset field is
    /// not 0
    pub fn is_fragmented(&self) -> bool {
        self.mf() || self.offset() != 0
    }

    /// Check if the packet is a multicast packet by looking at the destination address
    pub fn is_multicast(&self) -> bool {
        self.dest_addr().is_multicast()
    }

    /// Get the TTL (Time To Live) value
    pub fn ttl(&self) -> u8 {
        self.data[8]
    }

    /// Get the value of the protocol field
    pub fn protocol(&self) -> u8 {
        self.data[9]
    }

    /// Get the checksum field
    pub fn checksum(&self) -> u16 {
        super::util::read_int!(self.data, u16, 10)
    }

    /// Get the source address
    pub fn source_addr(&self) -> rist_rs_core::net::Ipv4Addr {
        [self.data[12], self.data[13], self.data[14], self.data[15]].into()
    }

    /// Get the destination address
    pub fn dest_addr(&self) -> rist_rs_core::net::Ipv4Addr {
        [self.data[16], self.data[17], self.data[18], self.data[19]].into()
    }

    /// Get all options as a slice of data. Returns an error if there is not enough data.
    pub fn options(&self) -> Result<&'a [u8], error::Error> {
        self.valid_header_len()
            .map(|len| &self.data[IPV4_BASE_HEADER_LEN..len])
    }

    /// Get the payload. Returns an error if there is not enough data
    pub fn payload(&self) -> Result<&'a [u8], error::Error> {
        self.valid_header_len()
            .and_then(|len| Ok((len, self.valid_total_len()?)))
            .and_then(|(header, total)| {
                if header > total {
                    Err(error::header_to_long())
                } else {
                    Ok(&self.data[header..total])
                }
            })
    }
}

// Implements display for Ipv4PacketView to pretty print packets
mod display;

// Tests
mod test;
