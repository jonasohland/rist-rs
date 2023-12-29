use crate::util::read_int;

pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        EndOfPacketReached,
    }
}

pub const SUBTYPE_RTT_ECHO_REQ: u8 = 2;
pub const SUBTYPE_RTT_ECHO_RES: u8 = 3;

#[derive(Debug, Clone, Copy)]
pub struct EchoMessageView<'a> {
    data: &'a [u8],
}

impl<'a> EchoMessageView<'a> {
    const PACKET_LEN_MIN: usize = 12;

    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() < Self::PACKET_LEN_MIN {
            Err(error::Error::EndOfPacketReached)
        } else {
            Ok(Self { data })
        }
    }

    pub fn timestamp(&self) -> rist_rs_types::time::ntp::Timestamp {
        rist_rs_types::time::ntp::Timestamp::new(
            read_int!(self.data, u32, 0),
            read_int!(self.data, u32, 4),
        )
    }

    pub fn unmarshal(&self) -> EchoMessage {
        EchoMessage {
            timestamp: self.timestamp(),
        }
    }
}

pub struct EchoMessage {
    timestamp: rist_rs_types::time::ntp::Timestamp,
}

impl EchoMessage {
    pub fn timestamp(&self) -> rist_rs_types::time::ntp::Timestamp {
        self.timestamp
    }
}
