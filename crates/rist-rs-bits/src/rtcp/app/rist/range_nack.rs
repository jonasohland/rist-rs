pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        InvalidPacketLength,
    }
}

pub const SUBTYPE_RANGE_NACK: u8 = 0;

#[derive(Debug, Clone, Copy)]
pub struct RangeNackMessage<'a> {
    data: &'a [u8],
}

pub struct PacketRangeRequest {
    pub seq_start: u16,
    pub count: u16,
}

impl From<[u8; 4]> for PacketRangeRequest {
    fn from(data: [u8; 4]) -> Self {
        Self {
            seq_start: u16::from_be_bytes([data[0], data[1]]),
            count: u16::from_be_bytes([data[2], data[3]]),
        }
    }
}

impl<'a> RangeNackMessage<'a> {
    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() % 4 != 0 {
            Err(error::Error::InvalidPacketLength)
        } else {
            Ok(Self { data })
        }
    }

    pub fn requests(&self) -> impl Iterator<Item = PacketRangeRequest> + 'a {
        self.data.chunks_exact(4).map(|slice| {
            let data: [u8; 4] = slice
                .try_into()
                .expect(rist_rs_core::internal::INTERNAL_ERR_PRE_VALIDATED);
            PacketRangeRequest::from(data)
        })
    }
}
