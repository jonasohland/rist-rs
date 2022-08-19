macro_rules! string_like_attribute {
    ($type_name:tt, $attr_name:literal) => {
        pub struct $type_name(String);
        impl crate::model::attribute::Attribute for $type_name {
            const NAME: &'static str = $attr_name;

            fn decode(s: &str) -> Self {
                Self(s.to_string())
            }

            fn encode(&self, writer: &mut impl core::fmt::Write) -> std::fmt::Result {
                write!(writer, "{}", self.0)
            }
        }
    };
}

string_like_attribute!(Charset, "charset");
string_like_attribute!(Type, "type");
string_like_attribute!(Category, "cat");
string_like_attribute!(Keywords, "keywds");
