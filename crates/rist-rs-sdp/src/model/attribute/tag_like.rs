#![allow(unused)]

use super::Attribute;

macro_rules! tag_like_attribute {
    ($type_name:tt, $attr_name:literal) => {
        pub struct $type_name();
        impl crate::model::attribute::Attribute for $type_name {
            const NAME: &'static str = $attr_name;

            fn decode(_: &str) -> Self {
                Self()
            }

            fn encode(&self, _: &mut impl core::fmt::Write) -> std::fmt::Result {
                Ok(())
            }
        }
    };
}

tag_like_attribute!(RecvOnly, "recvonly");
tag_like_attribute!(SendRecv, "sendrecv");
tag_like_attribute!(Send, "send");
tag_like_attribute!(Inactive, "inactive");
