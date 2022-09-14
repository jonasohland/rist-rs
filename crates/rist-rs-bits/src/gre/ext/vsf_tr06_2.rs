#[derive(Debug)]
pub struct Extension {
    bytes: [u8; 2],
}

impl From<[u8; 2]> for Extension {
    fn from(bytes: [u8; 2]) -> Self {
        Self { bytes }
    }
}

impl super::Extension0 for Extension {}

impl Extension {
    /// RIST GRE version used. If the version is 1, the key length can be obtained from this extension
    pub fn rist_gre_version(&self) -> u8 {
        (self.bytes[1] >> 3) & 0x7
    }

    /// Key length for PSK operation. Returns `None` if the RIST GRE version is 0, returns either 128 or 256
    /// if the RIST GRE version is 1
    pub fn key_length(&self) -> Option<u16> {
        (self.rist_gre_version() == 1).then(|| {
            if (self.bytes[1] & 0x40) != 0 {
                256
            } else {
                128
            }
        })
    }
}
