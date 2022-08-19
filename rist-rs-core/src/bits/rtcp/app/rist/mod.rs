pub mod rtt;
pub mod range_nack;

pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        UnknownSubtype(u8),
        EndOfPacketReached,
        RTT(super::rtt::error::Error),
        RangeNack(super::range_nack::error::Error)
    }

    impl From<super::rtt::error::Error> for Error {
        fn from(e: super::rtt::error::Error) -> Self {
            Error::RTT(e)
        }
    }

    impl From<super::range_nack::error::Error> for Error {
        fn from(e: super::range_nack::error::Error) -> Self {
            Error::RangeNack(e)
        }
    }

}

#[derive(Debug, Clone, Copy)]
pub enum RistApplicationSpecificMessage<'a> {
    RTTEchoRequest(rtt::EchoMessage<'a>),
    RTTEchoResponse(rtt::EchoMessage<'a>),
    RangeNack(range_nack::RangeNackMessage<'a>)
}

impl<'a> RistApplicationSpecificMessage<'a> {
    pub fn try_new<T, U>(subtype: u8, bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        match subtype {
            range_nack::SUBTYPE_RANGE_NACK => Ok(RistApplicationSpecificMessage::RangeNack(
                range_nack::RangeNackMessage::try_new(bytes)?
            )),
            rtt::SUBTYPE_RTT_ECHO_REQ => Ok(RistApplicationSpecificMessage::RTTEchoRequest(
                rtt::EchoMessage::try_new(bytes)?,
            )),
            rtt::SUBTYPE_RTT_ECHO_RES => Ok(RistApplicationSpecificMessage::RTTEchoResponse(
                rtt::EchoMessage::try_new(bytes)?,
            )),
            unknown => Err(error::Error::UnknownSubtype(unknown)),
        }
    }
}
