use core::str::{from_utf8, Utf8Error};

pub mod rist;

pub mod error {
    use core::str::Utf8Error;

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        EndOfPacketReached,
        UnknownApplication([u8; 4]),
        Utf8Error(Utf8Error),
        Rist(super::rist::error::Error),
    }

    impl From<Utf8Error> for Error {
        fn from(e: Utf8Error) -> Self {
            Self::Utf8Error(e)
        }
    }

    impl From<super::rist::error::Error> for Error {
        fn from(e: super::rist::error::Error) -> Self {
            Self::Rist(e)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageView<'a> {
    Rist(rist::RistApplicationSpecificMessageView<'a>),
}

#[derive(Debug, Clone, Copy)]
pub struct ApplicationSpecificMessageView<'a> {
    data: &'a [u8],
    subtype: u8,
}

impl<'a> TryFrom<(u8, &'a [u8])> for ApplicationSpecificMessageView<'a> {
    type Error = error::Error;

    fn try_from(value: (u8, &'a [u8])) -> Result<Self, Self::Error> {
        ApplicationSpecificMessageView::try_new(value.0, value.1)
    }
}

impl<'a> ApplicationSpecificMessageView<'a> {
    const SSRC_OFFSET: usize = 0;
    const NAME_OFFSET: usize = Self::SSRC_OFFSET + 4;
    const NAME_LEN: usize = 4;
    const DATA_OFFSET: usize = Self::NAME_OFFSET + Self::NAME_LEN;

    pub fn try_new<T, U>(aux: u8, bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() < 8 {
            Err(error::Error::EndOfPacketReached)
        } else {
            Ok(Self { subtype: aux, data })
        }
    }

    pub fn ssrc(&self) -> u32 {
        u32::from_be_bytes([
            self.data[Self::SSRC_OFFSET],
            self.data[Self::SSRC_OFFSET + 1],
            self.data[Self::SSRC_OFFSET + 2],
            self.data[Self::SSRC_OFFSET + 3],
        ])
    }

    pub fn name_tag(&self) -> [u8; 4] {
        [
            self.data[Self::NAME_OFFSET],
            self.data[Self::NAME_OFFSET + 1],
            self.data[Self::NAME_OFFSET + 2],
            self.data[Self::NAME_OFFSET + 3],
        ]
    }

    pub fn name(&self) -> Result<&'a str, Utf8Error> {
        from_utf8(&self.data[Self::NAME_OFFSET..Self::NAME_OFFSET + Self::NAME_LEN])
    }

    pub fn subtype(&self) -> u8 {
        self.subtype
    }

    pub fn message(&self) -> Result<MessageView<'a>, error::Error> {
        self.name()
            .map_err(error::Error::from)
            .and_then(|name| match name {
                "RIST" => Ok(MessageView::Rist(
                    rist::RistApplicationSpecificMessageView::try_new(
                        self.subtype(),
                        &self.data[Self::DATA_OFFSET..],
                    )?,
                )),
                _ => Err(error::Error::UnknownApplication(self.name_tag())),
            })
    }
}
