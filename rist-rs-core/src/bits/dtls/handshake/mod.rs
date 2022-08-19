use crate::internal::INTERNAL_ERR_PRE_VALIDATED;
use alloc::alloc::{Allocator, Global};

pub mod error;

mod certificate;
mod certificate_request;
mod certificate_verify;
mod client_hello;
mod client_key_exchange;
mod finished;
mod hello_request;
mod hello_verify_request;
mod server_hello;
mod server_hello_done;
mod server_key_exchange;

const MSG_TYPE_HELLO_REQUEST: u8 = 0;
const MSG_TYPE_CLIENT_HELLO: u8 = 1;

pub struct MessageView<'a> {
    data: &'a [u8],
}

pub enum HandshakeMessageBody<'a> {
    HelloRequest(hello_request::MessageView<'a>),
    ClientHello(client_hello::MessageView<'a>),
    HelloVerifyRequest(hello_verify_request::MessageView<'a>),
    ServerHello(server_hello::MessageView<'a>),
    Certificate(certificate::MessageView<'a>),
    ServerKeyExchange(server_key_exchange::MessageView<'a>),
    CertificateRequest(certificate_request::MessageView<'a>),
    ServerHelloDone(server_hello_done::MessageView<'a>),
    CertificateVerify(certificate_verify::MessageView<'a>),
    ClientKeyExchange(client_key_exchange::MessageView<'a>),
    Finished(finished::MessageView<'a>),
}

#[derive(Debug)]
pub struct Fragment<A: Allocator> {
    total_len: usize,
    offset: usize,
    body: Vec<u8, A>,
}

impl<'a> MessageView<'a> {
    const HEADER_LEN: usize = 12;

    pub fn try_new<T, U>(bytes: &'a T) -> Result<MessageView<'a>, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &[u8] = bytes.as_ref().into();
        if data.len() <= 11 {
            Err(error::too_small(data.len()))
        } else {
            Ok(MessageView { data })
        }
    }

    pub fn message_type(&self) -> u8 {
        self.data[0]
    }

    pub fn length(&self) -> usize {
        (u32::from_be_bytes(
            self.data[0..4]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        ) & 0xffffff) as usize
    }

    pub fn sequence_number(&self) -> u16 {
        u16::from_be_bytes(
            self.data[4..6]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        )
    }

    pub fn fragment_offset(&self) -> usize {
        (u32::from_be_bytes(
            self.data[6..10]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        ) >> 8) as usize
    }

    pub fn fragment_length(&self) -> usize {
        (u32::from_be_bytes(
            self.data[8..12]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        ) & 0x00ffffff) as usize
    }

    pub fn valid_fragment_length(&self) -> Result<usize, error::Error> {
        let l = self.fragment_length();
        if (l != self.data.len() - Self::HEADER_LEN) {
            Err(error::invalid_length(l, self.data.len() - Self::HEADER_LEN))
        } else {
            Ok(l)
        }
    }

    pub fn fragment(&self) -> Result<Fragment<Global>, error::Error> {
        self.fragment_in(Global::default())
    }

    pub fn fragment_in<A: Allocator>(&self, alloc: A) -> Result<Fragment<A>, error::Error> {
        let mut body = Vec::new_in(alloc);
        let l = self.valid_fragment_length()?;
        body.reserve_exact(l);
        body.extend_from_slice(&self.data[Self::HEADER_LEN..l + Self::HEADER_LEN]);
        Ok(Fragment {
            total_len: self.length(),
            offset: self.fragment_offset(),
            body,
        })
    }

    pub fn body(&self) -> Result<HandshakeMessageBody, error::Error> {
        match self.data[0] {
            MSG_TYPE_HELLO_REQUEST => Ok(HandshakeMessageBody::HelloRequest(
                hello_request::MessageView::try_new(&self.data[11..])?,
            )),
            _ => Err(error::unknown_body_type(self.data[10])),
        }
    }
}


