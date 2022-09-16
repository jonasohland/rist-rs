#![allow(unused)]
use super::Attribute;

enum RtpMapExt {
    AudioChannels(usize),
}

pub struct RtpMap {
    pt: u8,
    enc_name: String,
    clock_rate: u32,
    ext: Option<RtpMapExt>,
}

impl RtpMap {
    fn new(pt: u8, enc_name: String, clock_rate: u32) -> Self {
        Self {
            pt,
            enc_name,
            clock_rate,
            ext: None,
        }
    }

    fn new_audio(pt: u8, enc_name: String, clock_rate: u32, channels: usize) -> Self {
        Self {
            pt,
            enc_name,
            clock_rate,
            ext: Some(RtpMapExt::AudioChannels(channels)),
        }
    }
}

impl Attribute for RtpMap {
    const NAME: &'static str = "rtpmap";

    fn decode(text: &str) -> Self {
        todo!()
    }

    fn encode(&self, writer: &mut impl core::fmt::Write) -> std::fmt::Result {
        write!(writer, "{} {}/{}", self.pt, self.enc_name, self.clock_rate)?;
        if let Some(RtpMapExt::AudioChannels(ch)) = self.ext {
            write!(writer, "/{}", ch)?;
        }
        Ok(())
    }
}
