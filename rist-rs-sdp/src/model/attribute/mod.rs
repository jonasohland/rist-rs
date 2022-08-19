pub trait Attribute {
    const NAME: &'static str;
    fn decode(text: &str) -> Self;
    fn encode(&self, writer: &mut impl core::fmt::Write) -> std::fmt::Result;
}


mod rtp_map;
mod tag_like;
mod string_like;

pub use rtp_map::RtpMap;
pub use tag_like::Inactive;
pub use tag_like::RecvOnly;
pub use tag_like::Send;
pub use tag_like::SendRecv;
pub use string_like::Category;
pub use string_like::Charset;
pub use string_like::Type;
pub use string_like::Keywords;