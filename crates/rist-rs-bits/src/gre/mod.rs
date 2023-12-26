#![allow(unused)]
pub mod ext;

use core::{
    convert::{TryFrom, TryInto},
    mem::size_of,
};

use super::util;

#[derive(Debug)]
struct Error {}

struct GREPacket<'a> {
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for GREPacket<'a> {
    type Error = Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_new(data)
    }
}

impl<'a> GREPacket<'a> {
    /// Offset of the optional fields after the fixed header
    const OPT_FIELDS_OFFSET: usize = 4;

    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() >= 4 {
            Ok(Self { data })
        } else {
            Err(Error {})
        }
    }

    /// Check if the checksum bit is set
    fn has_checksum(&self) -> bool {
        util::check_bit!(self.data[0], 0)
    }

    /// Check if the key field is set
    fn has_key(&self) -> bool {
        util::check_bit!(self.data[0], 2)
    }

    /// Check if the sequence number bit is set
    fn has_sequence(&self) -> bool {
        util::check_bit!(self.data[0], 3)
    }

    /// Get the GRE protocol version
    fn version(&self) -> u8 {
        self.data[1] & 0x7
    }

    /// Get the encapsulated protocol type
    fn protocol(&self) -> u16 {
        util::read_int!(self.data, u16, 2)
    }

    /// Get the checksum. Returns `None` if the checksum bit is not set, an `Error` if the
    /// slice is too short to contain a checksum at the right position, or the checksum
    fn checksum(&self) -> Option<Result<u16, Error>> {
        self.has_checksum().then(|| {
            if self.data.len() < Self::OPT_FIELDS_OFFSET + size_of::<u32>() {
                Err(Error {})
            } else {
                Ok(util::read_int!(self.data, u16, Self::OPT_FIELDS_OFFSET))
            }
        })
    }

    /// Get the key. Returns `None` if the key bit is not set, an `Error` if the
    /// slice is too short to contain a key at the right position, or the key
    fn key(&self) -> Option<Result<u32, Error>> {
        self.has_key().then(|| {
            let offset = if self.has_checksum() {
                Self::OPT_FIELDS_OFFSET + size_of::<u32>()
            } else {
                Self::OPT_FIELDS_OFFSET
            };
            if self.data.len() < offset + size_of::<u32>() {
                Err(Error {})
            } else {
                Ok(util::read_int!(self.data, u32, offset))
            }
        })
    }

    /// Get the sequence number. Returns `None` if the sequence number bit is not set,
    /// an `Error` if the slice is too short to contain a sequence number at the right position,
    /// or the sequence number
    fn sequence_number(&self) -> Option<Result<u32, Error>> {
        self.has_sequence().then(|| {
            let mut offset = Self::OPT_FIELDS_OFFSET;
            if self.has_checksum() {
                offset += size_of::<u32>();
            }
            if self.has_key() {
                offset += size_of::<u32>();
            }
            if self.data.len() < offset + size_of::<u32>() {
                Err(Error {})
            } else {
                Ok(util::read_int!(self.data, u32, offset))
            }
        })
    }

    /// Get the encapsulated payload. Returns an error if the slice is shorter than the expected header length.
    /// Otherwise returns the (possibly zero-sized) payload
    fn payload(&self) -> Result<&'a [u8], Error> {
        let mut offset = Self::OPT_FIELDS_OFFSET;
        if self.has_checksum() {
            offset += size_of::<u32>();
        }
        if self.has_key() {
            offset += size_of::<u32>();
        }
        if self.has_sequence() {
            offset += size_of::<u32>();
        }
        if self.data.len() < offset {
            Err(Error {})
        } else {
            Ok(&self.data[offset..])
        }
    }

    /// Verify the checksum. The checksum implementation behind this function is very naive and slow
    /// and should not be used in production scenarios
    fn verify_checksum(&self) -> Option<Result<bool, Error>> {
        self.checksum().map(|sum| {
            sum.map(|sum| util::checksum::u16(self.data, Some(Self::OPT_FIELDS_OFFSET)) == sum)
        })
    }

    /// Get the first extension field
    fn ext0<Ext: ext::Extension0>(&self) -> Ext {
        Ext::from(
            self.data[0..2]
                .try_into()
                .expect("invalid range returned from slice"),
        )
    }

    /// Get the second extension field
    fn ext1<Ext: ext::Extension1>(&self) -> Option<Result<Ext, Error>> {
        self.has_checksum().then(|| {
            if self.data.len() < Self::OPT_FIELDS_OFFSET + size_of::<u32>() {
                Err(Error {})
            } else {
                Ok(Ext::from(
                    self.data[Self::OPT_FIELDS_OFFSET + size_of::<u16>()
                        ..Self::OPT_FIELDS_OFFSET + size_of::<u32>()]
                        .try_into()
                        .expect("invalid range returned from slice"),
                ))
            }
        })
    }
}

mod test {

    use super::*;

    const EXAMPLE_GRE: [u8; 24] = [
        // GRE header
        0x00, 0x00, 0x08, 0x00, // IP Header
        0x45, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x00, 0x00, 0xff, 0x01, 0xb5, 0x89, 0x01, 0x01, 0x01,
        0x01, 0x02, 0x02, 0x02, 0x02,
    ];

    const GRE_WITH_KEY: [u8; 8] = [0x20, 0x00, 0x01, 0x01, 0x11, 0x11, 0x11, 0x0a];

    fn packet(data: &[u8]) -> GREPacket {
        GREPacket::try_from(data).unwrap()
    }

    #[test]
    fn test() {
        let data: [u8; 12] = [
            // 0x00, 0x00, 0x00, 0x00
            0x30, 0x08, 0x88, 0xb6, 0x0c, 0xcd, 0x63, 0x8a, 0x00, 0x00, 0x04, 0x40,
        ];

        let gre = GREPacket::try_from(data.as_slice()).unwrap();

        println!("{:#02x}", gre.protocol());
        println!("{}", gre.version());
        println!(
            "{:?}",
            gre.checksum().map(|s| s.map(|k| format!("{k:#02x}")))
        );
        println!("{:?}", gre.key().map(|s| s.map(|k| format!("{k:#04x}"))));
        println!("{:?}", gre.sequence_number());
        println!("{:?}", gre.payload());
        println!(
            "{:?}",
            gre.ext0::<ext::vsf_tr06_2::Extension>().rist_gre_version()
        );
        println!(
            "{:?}",
            gre.ext0::<ext::vsf_tr06_2::Extension>().key_length()
        );
    }

    #[test]
    fn gre_basic() {
        let gre = packet(&EXAMPLE_GRE);
        assert!(!gre.has_checksum());
        assert!(!gre.has_key());
        assert!(!gre.has_sequence());
        assert_eq!(gre.version(), 0);
        assert!(gre.checksum().is_none());
        assert!(gre.sequence_number().is_none());
        assert!(gre.key().is_none());
    }

    #[test]
    fn gre_key() {
        let gre = packet(&GRE_WITH_KEY);
        assert!(gre.has_key());
        assert_eq!(gre.key().unwrap().unwrap(), 0x1111110a);
    }
}
