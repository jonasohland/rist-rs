use core::borrow::Borrow;

pub trait ReadNonBlocking {
    type Error;
    fn try_read(&mut self, buf: &mut [u8]) -> Option<Result<usize, Self::Error>>;
}

pub trait WriteNonBlocking {
    type Error;
    fn try_write(&mut self, buf: &[u8]) -> Option<Result<usize, Self::Error>>;
}

pub trait ReceiveNonBlocking {
    type Error;
    fn try_recv(&mut self, buf: &mut [u8]) -> Option<Result<usize, Self::Error>>;
}

pub trait ReceiveFromNonBlocking {
    type Error;
    type Address;

    #[allow(clippy::type_complexity)]
    fn try_recv_from(
        &mut self,
        buf: &mut [u8],
    ) -> Option<Result<(usize, Self::Address), Self::Error>>;
}

pub trait SendNonBlocking {
    type Error;
    fn try_send(&mut self, buf: &[u8]) -> Option<Result<usize, Self::Error>>;
}

pub trait SendToNonBlocking {
    type Error;
    type Address;
    fn try_send_to<A: Borrow<Self::Address>>(
        &mut self,
        buf: &[u8],
        address: A,
    ) -> Option<Result<usize, Self::Error>>;
}
