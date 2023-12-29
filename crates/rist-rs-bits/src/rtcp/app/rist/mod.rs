use self::{range_nack::RangeNackMessage, rtt::EchoMessage};

pub mod range_nack;
pub mod rtt;

pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        UnknownSubtype(u8),
        EndOfPacketReached,
        RTT(super::rtt::error::Error),
        RangeNack(super::range_nack::error::Error),
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
pub enum RistApplicationSpecificMessageView<'a> {
    RTTEchoRequest(rtt::EchoMessageView<'a>),
    RTTEchoResponse(rtt::EchoMessageView<'a>),
    RangeNack(range_nack::RangeNackMessageView<'a>),
}

impl<'a> RistApplicationSpecificMessageView<'a> {
    pub fn try_new<T, U>(subtype: u8, bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        match subtype {
            range_nack::SUBTYPE_RANGE_NACK => Ok(RistApplicationSpecificMessageView::RangeNack(
                range_nack::RangeNackMessageView::try_new(bytes)?,
            )),
            rtt::SUBTYPE_RTT_ECHO_REQ => Ok(RistApplicationSpecificMessageView::RTTEchoRequest(
                rtt::EchoMessageView::try_new(bytes)?,
            )),
            rtt::SUBTYPE_RTT_ECHO_RES => Ok(RistApplicationSpecificMessageView::RTTEchoResponse(
                rtt::EchoMessageView::try_new(bytes)?,
            )),
            unknown => Err(error::Error::UnknownSubtype(unknown)),
        }
    }

    pub fn unmarshal(&self) -> RistApplicationSpecificMessage {
        match self {
            RistApplicationSpecificMessageView::RTTEchoRequest(req) => {
                RistApplicationSpecificMessage::RTTEchoRequest(req.unmarshal())
            }
            RistApplicationSpecificMessageView::RTTEchoResponse(res) => {
                RistApplicationSpecificMessage::RTTEchoResponse(res.unmarshal())
            }
            RistApplicationSpecificMessageView::RangeNack(rn) => {
                RistApplicationSpecificMessage::RangeNack(rn.unmarshal())
            }
        }
    }
}

pub enum RistApplicationSpecificMessage {
    RTTEchoRequest(EchoMessage),
    RTTEchoResponse(EchoMessage),
    RangeNack(RangeNackMessage),
}
