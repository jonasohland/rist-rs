#![allow(unused)]
pub mod error;
mod ext;

use super::util;
use core::convert::TryFrom;

#[derive(Debug, Clone)]
pub struct RTPView<'a> {
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for RTPView<'a> {
    type Error = error::Error;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        RTPView::try_new(value)
    }
}

impl<'a> RTPView<'a> {
    pub fn try_new<T, U>(buf: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = buf.as_ref().into();
        if data.len() < 12 {
            Err(error::other("length must be at least 12 bytes"))
        } else {
            Ok(RTPView { data })
        }
    }

    /// Minimum header length is 12 bytes
    const HEADER_LEN_MIN: usize = 12;

    /// Get the rtp protocol version. This almost guaranteed to be 2
    pub fn version(&self) -> u8 {
        (self.data[0] & 0xc0) >> 6
    }

    /// Check if the header extension bit is set
    pub fn has_extension(&self) -> bool {
        (self.data[0] & 0x10) != 0
    }

    /// Check if the padding bit is set
    pub fn has_padding(&self) -> bool {
        (self.data[0] & 0x20) != 0
    }

    /// Get the SSRC value for the stream this packet belongs to
    pub fn ssrc(&self) -> u32 {
        util::read_int!(self.data, u32, 8)
    }

    /// Get the (not extended) sequence number
    pub fn sequence_number(&self) -> u16 {
        util::read_int!(self.data, u16, 2)
    }

    /// Get the RTP timestamp
    pub fn timestamp(&self) -> u32 {
        util::read_int!(self.data, u32, 4)
    }

    /// Get the number of csrcs in this packet
    pub fn csrc_count(&self) -> u8 {
        self.data[0] & 0xf
    }

    fn crscs_len(&self) -> usize {
        self.csrc_count() as usize * core::mem::size_of::<u32>()
    }

    /// Get an iterator over the csrc's in this packet
    pub fn csrc(&self) -> impl Iterator<Item = u32> + '_ {
        self.data[Self::HEADER_LEN_MIN..]
            .chunks_exact(4)
            .take(self.csrc_count() as usize)
            .map(|b| util::read_int!(b, u32, 0))
    }

    /// Get the length ob the padding added to the payload.
    /// Return `None` if the padding bit is not set and an error
    /// if the padding length is zero (which makes no sense since the
    /// padding-length value is part of the padding itself)
    fn padding_len(&self) -> Option<Result<u8, error::Error>> {
        self.has_padding()
            .then(|| match self.data[self.data.len() - 1] {
                0 => Err(error::other("invalid padding length, cannot be 0")),
                l if l <= self.data.len() as u8 - Self::HEADER_LEN_MIN as u8 => Ok(l),
                _s => Err(error::other("invalid padding")),
            })
    }

    pub fn payload(&self) -> Result<&'a [u8], error::Error> {
        let offset =
            Self::HEADER_LEN_MIN + self.extension_len().unwrap_or(Ok(0))? + self.crscs_len();
        let padding = self.padding_len().unwrap_or(Ok(0))? as usize;
        if offset + padding > self.data.len() {
            Err(error::other("length error"))
        } else {
            Ok(&self.data[offset..self.data.len() - padding])
        }
    }

    /// Get the extension information header defined in RFC3550 5.3.1
    /// Returns `None` if the extension bit is not set
    pub fn extension_info(&self) -> Option<Result<([u8; 2], u16), error::Error>> {
        const FIELD_LEN: usize = 4;
        self.has_extension().then(|| {
            let offset = Self::HEADER_LEN_MIN + self.crscs_len();
            if self.data.len() >= offset + FIELD_LEN {
                Ok((
                    util::into_array(&self.data[offset..offset + 2]),
                    util::read_int!(self.data, u16, offset + 2),
                ))
            } else {
                Err(error::other("not good"))
            }
        })
    }

    /// Length of the full extension field. `None` if the extension bit is not set
    pub fn extension_len(&self) -> Option<Result<usize, error::Error>> {
        self.extension_info()
            .map(|r| r.map(|(_, len)| len as usize))
    }

    /// Get the extension header. Returns `None` if the extension bit is not set
    /// and an error if conversion to the extension type fails.
    fn extension<'b, Ext: ext::ReadExt<'a>>(&self) -> Option<Result<Ext, Ext::Error>> {
        self.has_extension().then(|| {
            Ext::try_from(&self.data[Self::HEADER_LEN_MIN + (4 * self.csrc_count() as usize)..])
        })
    }
}

trait RTPWriter {
    fn sequence_number() -> u16;
}

mod test {

    use super::*;

    /// Packet with 2 bytes of payload
    const SOME_PACKET: [u8; 14] = [
        0x80, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x47, 0x40,
    ];

    /// Empty packet
    const RTP_WITH_0_LEN: [u8; 12] = [
        0x80, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2,
    ];

    /// Empty packet with 1 byte of padding
    const RTP_0_LEN_PADDED_1: [u8; 13] = [
        0xA0, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x01,
    ];

    /// Empty packet with 2 bytes padding
    const RTP_0_LEN_PADDED_2: [u8; 14] = [
        0xA0, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x00, 0x02,
    ];

    /// Empty packet with 4 crscs
    const RTP_0_LEN_4_CSRC: [u8; 12 + 16] = [
        0x84, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];

    /// Empty packet with 4 csrcs and 4 bytes of padding
    const RTP_0_LEN_4_CSRC_4_PADDING: [u8; 12 + 16 + 4] = [
        0xA4, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0xff, 0xff,
        0xff, 0x04,
    ];

    /// Packet with 16 bytes payload  4 csrcs and 4 bytes of padding
    const RTP_16_LEN_4_CSRC_4_PADDING: [u8; 12 + 16 + 16 + 4] = [
        0xA4, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
        // payload
        0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff,
        0xfa, // padding
        0xff, 0xff, 0xff, 0x04,
    ];

    /// Packet with 2 bytes of payload
    const RTP_INVALID_LEN: [u8; 11] = [
        0x80, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe,
    ];

    /// Empty packet
    const RTP_BROKEN_PADDING: [u8; 13] = [
        0xA0, 0x21, 0x23, 0x6c, 0x5b, 0x68, 0x20, 0x88, 0xb3, 0x59, 0xbe, 0xe2, 0x00,
    ];

    pub fn packet(data: &[u8]) -> RTPView {
        RTPView::try_from(data).unwrap()
    }

    #[test]
    fn basics() {
        let rtp = packet(&SOME_PACKET);
        assert_eq!(rtp.version(), 2);
        assert_eq!(rtp.ssrc(), 3009003234);
        assert_eq!(rtp.timestamp(), 1533550728);
        assert_eq!(rtp.sequence_number(), 9068);
        assert!(!rtp.has_extension());
        assert!(!rtp.has_padding());
        assert_eq!(rtp.payload().unwrap(), &[0x47u8, 0x40u8])
    }

    #[test]
    fn empty() {
        let rtp = packet(&RTP_WITH_0_LEN);
        assert_eq!(rtp.payload().unwrap().len(), 0);
    }

    #[test]
    fn empty_with_padding() {
        let padding_l1 = packet(&RTP_0_LEN_PADDED_1);
        let padding_l2 = packet(&RTP_0_LEN_PADDED_2);
        assert!(padding_l1.has_padding());
        assert!(padding_l2.has_padding());
        assert_eq!(padding_l1.padding_len().unwrap().unwrap(), 1);
        assert_eq!(padding_l2.padding_len().unwrap().unwrap(), 2);
        assert_eq!(padding_l1.payload().unwrap().len(), 0);
        assert_eq!(padding_l2.payload().unwrap().len(), 0);
    }

    #[test]
    fn empty_and_crscs() {
        let rtp = packet(&RTP_0_LEN_4_CSRC);
        assert_eq!(rtp.csrc_count(), 4);
        let csrc_v = rtp.csrc().collect::<Vec<_>>();
        assert_eq!(csrc_v[0], 0x1000000);
        assert_eq!(csrc_v[1], 0x10000);
        assert_eq!(csrc_v[2], 0x100);
        assert_eq!(csrc_v[3], 0x1);
    }

    #[test]
    fn empty_and_csrcs_and_padding() {
        let rtp = packet(&RTP_0_LEN_4_CSRC_4_PADDING);
        assert_eq!(rtp.csrc_count(), 4);
        let csrc_v = rtp.csrc().collect::<Vec<_>>();
        assert_eq!(csrc_v[0], 0x1000000);
        assert_eq!(csrc_v[1], 0x10000);
        assert_eq!(csrc_v[2], 0x100);
        assert_eq!(csrc_v[3], 0x1);
        assert!(rtp.has_padding());
        assert_eq!(rtp.payload().unwrap().len(), 0);
    }

    #[test]
    fn payload_and_padding_and_csrcs() {
        let rtp = packet(&RTP_16_LEN_4_CSRC_4_PADDING);
        assert_eq!(rtp.csrc_count(), 4);
        let csrc_v = rtp.csrc().collect::<Vec<_>>();
        assert_eq!(csrc_v[0], 0x1000000);
        assert_eq!(csrc_v[1], 0x10000);
        assert_eq!(csrc_v[2], 0x100);
        assert_eq!(csrc_v[3], 0x1);
        assert!(rtp.has_padding());
        assert_eq!(rtp.payload().unwrap().len(), 16);
        assert_eq!(
            rtp.payload().unwrap(),
            &[
                0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa, 0xff, 0xfa,
                0xff, 0xfa
            ]
        )
    }

    #[test]
    fn invalid() {
        // conversion from invalid length should fail
        assert!(matches!(
            RTPView::try_from(RTP_INVALID_LEN.as_slice()),
            Err(_)
        ));

        let broken_padding = packet(&RTP_BROKEN_PADDING);
        assert!(broken_padding.has_padding());
        assert!(matches!(broken_padding.payload(), Err(_)));
        assert!(matches!(broken_padding.padding_len().unwrap(), Err(_)))
    }
}
