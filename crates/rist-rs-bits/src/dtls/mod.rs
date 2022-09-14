#![allow(unused)]
mod app;
mod ccs;
mod error;
mod handshake;

use core::fmt::Display;

use rist_rs_core::internal::INTERNAL_ERR_PRE_VALIDATED;
use error::Error;

struct DTLSRecordView<'a> {
    data: &'a [u8],
}

enum Version {
    V1_0,
    V1_2,
}

impl Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Version::V1_0 => write!(f, "DTLS 1.0"),
            Version::V1_2 => write!(f, "DTLS 1.2"),
        }
    }
}

impl Version {
    fn new(bytes: [u8; 2]) -> Result<Version, error::Error> {
        match bytes {
            [254, 255] => Ok(Version::V1_0),
            [254, 253] => Ok(Version::V1_2),
            _ => Err(error::unknown_version(bytes)),
        }
    }

    fn bytes(&self) -> [u8; 2] {
        match self {
            Version::V1_0 => [254, 255],
            Version::V1_2 => [254, 253],
        }
    }
}

enum DTLSNextLayer<'a> {
    HandshakeProto(handshake::MessageView<'a>),
    ChangeCipherSpec(),
    AlertProto(),
    AppDataProto(),
}

impl<'a> DTLSRecordView<'a> {
    pub fn try_new<T, U>(bytes: &'a T) -> Result<DTLSRecordView<'a>, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &[u8] = bytes.as_ref().into();
        if data.len() <= 13 {
            Err(error::too_small(data.len()))
        } else {
            Ok(DTLSRecordView { data })
        }
    }

    pub fn version(&self) -> Result<Version, Error> {
        Version::new(
            self.data[1..3]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        )
    }

    pub fn epoch(&self) -> u16 {
        u16::from_be_bytes(
            self.data[3..5]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        )
    }

    pub fn sequence_number(&self) -> u64 {
        u64::from_be_bytes(
            self.data[5..13]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        ) >> 16
    }

    pub fn length(&self) -> usize {
        u16::from_be_bytes(
            self.data[11..13]
                .try_into()
                .expect(INTERNAL_ERR_PRE_VALIDATED),
        ) as usize
    }

    pub fn valid_length(&self) -> Result<usize, error::Error> {
        let l = self.length();
        if l != self.data.len() - 13 {
            Err(error::unexpected_length(l, self.data.len() - 13))
        } else {
            Ok(l)
        }
    }

    pub fn next_layer(&self) -> Result<DTLSNextLayer, Error> {
        match self.data[0] {
            20 => Ok(DTLSNextLayer::ChangeCipherSpec()),
            21 => Ok(DTLSNextLayer::AlertProto()),
            22 => Ok(DTLSNextLayer::HandshakeProto(
                handshake::MessageView::try_new(&self.data[13..self.valid_length()? + 13])?,
            )),
            23 => Ok(DTLSNextLayer::AppDataProto()),
            _ => Err(error::unknown_content_type(self.data[0])),
        }
    }
}

fn dtls_record_length(data: &[u8]) -> Result<usize, error::Error> {
    if data.len() < 13 {
        Err(error::too_small(data.len()))
    } else {
        Ok(u16::from_be_bytes(data[11..13].try_into().expect(INTERNAL_ERR_PRE_VALIDATED)) as usize)
    }
}

struct RecordIterator<'a> {
    pos: usize,
    data: &'a [u8],
}

impl<'a> Iterator for RecordIterator<'a> {
    type Item = Result<DTLSRecordView<'a>, error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.pos >= self.data.len()) {
            None
        } else {
            Some(
                dtls_record_length(&self.data[self.pos..])
                    .and_then(|l| {
                        if (l <= self.data.len()) {
                            let p = self.pos;
                            self.pos += l;
                            DTLSRecordView::try_new(&self.data[p..p + l])
                        } else {
                            Err(error::too_small(self.data.len()))
                        }
                    })
                    .map_err(|e| {
                        self.pos = usize::MAX;
                        e
                    }),
            )
        }
    }
}

mod test {
    use super::DTLSRecordView;

    const DTLS10_1: [u8; 164] = [
        0x16, 0xfe, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x97, 0x01, 0x00,
        0x00, 0x8b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8b, 0xfe, 0xff, 0x09, 0x98, 0xba,
        0x99, 0x1c, 0x8a, 0xaa, 0xd3, 0x05, 0x40, 0x85, 0x42, 0x28, 0x18, 0x90, 0xca, 0xf8, 0xcf,
        0x11, 0x07, 0xe6, 0xee, 0x7b, 0x9f, 0xe3, 0x52, 0x1a, 0xa3, 0x5a, 0x38, 0x6e, 0xd5, 0x00,
        0x00, 0x00, 0x58, 0xc0, 0x14, 0xc0, 0x0a, 0xc0, 0x22, 0xc0, 0x21, 0x00, 0x39, 0x00, 0x38,
        0x00, 0x88, 0x00, 0x87, 0xc0, 0x0f, 0xc0, 0x05, 0x00, 0x35, 0x00, 0x84, 0xc0, 0x12, 0xc0,
        0x08, 0xc0, 0x1c, 0xc0, 0x1b, 0x00, 0x16, 0x00, 0x13, 0xc0, 0x0d, 0xc0, 0x03, 0x00, 0x0a,
        0xc0, 0x13, 0xc0, 0x09, 0xc0, 0x1f, 0xc0, 0x1e, 0x00, 0x33, 0x00, 0x32, 0x00, 0x9a, 0x00,
        0x99, 0x00, 0x45, 0x00, 0x44, 0xc0, 0x0e, 0xc0, 0x04, 0x00, 0x2f, 0x00, 0x96, 0x00, 0x41,
        0x00, 0x15, 0x00, 0x12, 0x00, 0x09, 0x00, 0x14, 0x00, 0x11, 0x00, 0x08, 0x00, 0x06, 0x00,
        0xff, 0x01, 0x00, 0x00, 0x09, 0x00, 0x23, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x01, 0x01,
    ];

    #[test]
    fn parse_test() {
        let view = DTLSRecordView::try_new(&DTLS10_1).unwrap();
        println!("{}", view.version().unwrap());
        println!("{}", view.epoch());
        println!("{}", view.sequence_number());
        println!("{}", view.length());
        match view.next_layer().unwrap() {
            crate::dtls::DTLSNextLayer::HandshakeProto(h) => {
                println!("{} {:?}", h.message_type(), h.valid_fragment_length());
                println!("{}", h.fragment_offset());
                println!("{}", h.fragment_length());
            }
            crate::dtls::DTLSNextLayer::ChangeCipherSpec() => println!("ccp"),
            crate::dtls::DTLSNextLayer::AlertProto() => println!("alert"),
            crate::dtls::DTLSNextLayer::AppDataProto() => println!("app data"),
        }
    }
}
