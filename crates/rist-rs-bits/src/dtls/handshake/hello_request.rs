use core::convert::Infallible;

pub struct MessageView<'a> {
    data: &'a [u8],
}

impl<'a> MessageView<'a> {

    pub fn try_new<T, U>(bytes: &'a T) -> Result<MessageView<'a>, super::error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        Ok(MessageView { data: bytes.as_ref().into() })
    }
}