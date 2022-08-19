pub mod app;
pub mod rr;
pub mod rx_report;
pub mod sdes;
pub mod sr;

pub mod error {

    #[derive(Debug)]
    pub enum ErrorKind {
        NotEnoughData {
            need: usize,
            got: usize,
            field: &'static &'static str,
        },
        InvalidPadding,
        UnknownReportType(u8),
        SDES(super::sdes::error::Error),
        RR(super::rr::error::Error),
        SR(super::sr::error::Error),
        APP(super::app::error::Error),
    }

    #[derive(Debug)]
    pub struct Error {
        kind: ErrorKind,
    }

    impl From<super::sdes::error::Error> for Error {
        fn from(e: super::sdes::error::Error) -> Self {
            Error {
                kind: ErrorKind::SDES(e),
            }
        }
    }

    impl From<super::rr::error::Error> for Error {
        fn from(e: super::rr::error::Error) -> Self {
            Error {
                kind: ErrorKind::RR(e),
            }
        }
    }

    impl From<super::sr::error::Error> for Error {
        fn from(e: super::sr::error::Error) -> Self {
            Error {
                kind: ErrorKind::SR(e),
            }
        }
    }

    impl From<super::app::error::Error> for Error {
        fn from(e: super::app::error::Error) -> Self {
            Error {
                kind: ErrorKind::APP(e),
            }
        }
    }

    pub fn not_enough_data(need: usize, got: usize, field: &'static &'static str) -> Error {
        Error {
            kind: ErrorKind::NotEnoughData { need, got, field },
        }
    }

    pub fn invalid_padding() -> Error {
        Error {
            kind: ErrorKind::InvalidPadding,
        }
    }

    pub fn unknown_report_type(t: u8) -> Error {
        Error {
            kind: ErrorKind::UnknownReportType(t),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy)]
pub enum RTCPReportView<'a> {
    SR(sr::SenderReportMessageView<'a>),
    RR(rr::ReceiverReportMessageView<'a>),
    SDES(sdes::SourceDescriptionMessageIterator<'a>),
    APP(app::ApplicationSpecificMessageView<'a>),
    NACK(),
}

const RTCP_PT_SR: u8 = 200;
const RTCP_PT_RR: u8 = 201;
const RTCP_PT_SDES: u8 = 202;
const RTCP_PT_BYTE: u8 = 203;
const RTCP_PT_APP: u8 = 204;
const RTCP_PT_NACK: u8 = 205;

impl<'a> RTCPReportView<'a> {
    fn try_new<T, U>(packet_type: u8, aux: u8, bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        match packet_type {
            200 => Ok(RTCPReportView::SR(sr::SenderReportMessageView::try_new(
                bytes,
            )?)),
            201 => Ok(RTCPReportView::RR(rr::ReceiverReportMessageView::try_new(
                bytes,
            )?)),
            202 => Ok(RTCPReportView::SDES(
                sdes::SourceDescriptionMessageIterator::try_new(bytes)?,
            )),
            204 => Ok(RTCPReportView::APP(
                app::ApplicationSpecificMessageView::try_new(aux, bytes)?,
            )),
            _ => Err(error::unknown_report_type(packet_type)),
        }
    }
}

/// View over a single RTCP packet
#[derive(Debug, Clone, Copy)]
pub struct RTCPPacketView<'a> {
    data: &'a [u8],
}

/// Convert from a slice of bytes to a [RTCPPacketView]. Returns an error
/// if the slice can not possibly hold a full RTCP packet
impl<'a> TryFrom<&'a [u8]> for RTCPPacketView<'a> {
    type Error = error::Error;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_new(data)
    }
}

impl<'a> RTCPPacketView<'a> {
    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() < 8 {
            Err(error::not_enough_data(8, data.len(), &""))
        } else {
            Ok(RTCPPacketView { data })
        }
    }

    /// Total length of the packet as indicated by the Length field in bytes
    pub fn packet_len(&self) -> usize {
        (u16::from_be_bytes([self.data[2], self.data[3]]) as usize + 1) * 4
    }

    pub fn valid_packet_len(&self) -> Result<usize, error::Error> {
        let len = self.packet_len();
        if len > self.data.len() {
            Err(error::not_enough_data(len, self.data.len(), &"RTCPPacket"))
        } else {
            Ok(len)
        }
    }

    /// RTP version, should be 2 in most cases
    pub fn version(&self) -> u8 {
        (self.data[0] & 0xc0) >> 6
    }

    /// true if padding is added to the end of the padding
    pub fn padding(&self) -> bool {
        self.data[0] & 0x20 != 0
    }

    /// Get the padding length. Returns `None` if there is no padding.
    /// Returns an error if padding bit is set and the padding length is 0.
    /// Returns an error if the padding length is more than the internal slice length
    /// minus the RTCP header length
    pub fn padding_len(&self) -> Option<Result<usize, error::Error>> {
        self.padding().then(|| {
            self.valid_packet_len().and_then(|len| {
                let padding_len = self.data[len - 1] as usize;
                if padding_len == 0 || padding_len > (self.data.len() - 8) {
                    Err(error::invalid_padding())
                } else {
                    Ok(padding_len)
                }
            })
        })
    }

    /// Auxillary data. May be used as application-specific subtype in APP packets
    fn aux(&self) -> u8 {
        self.data[0] & 0x1f
    }

    /// Packet type
    fn pt(&self) -> u8 {
        self.data[1]
    }

    /// application specific subtype
    pub fn subtype(&self) -> Option<u8> {
        (self.pt() == RTCP_PT_APP).then(|| self.aux())
    }

    /// number items in the packet (reception reports, sdes items...)
    pub fn item_count(&self) -> Option<u8> {
        [RTCP_PT_RR, RTCP_PT_SR, RTCP_PT_SDES]
            .contains(&self.pt())
            .then(|| self.aux())
    }

    pub fn report(&self) -> Result<RTCPReportView<'a>, error::Error> {
        self.valid_packet_len()
            .and_then(|len| Ok((len, self.padding_len().unwrap_or(Ok(0))?)))
            .and_then(|(packet_len, padding_len)| {
                RTCPReportView::try_new(
                    self.pt(),
                    self.aux(),
                    &self.data[4..packet_len - padding_len],
                )
            })
    }
}

/// Can wrap any number of concatenated RTCP packets and provides
/// an implementation of [Iterator] that yields RTCP packets that are
/// parsed on-the-fly.
pub struct RTCPPacketViewIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RTCPPacketViewIterator<'a> {
    /// Creates a new [RTCPPacketViewIterator] from a reference to `T`.
    /// `T` must be convertible to a slice of bytes
    pub fn new<T, U>(bytes: &'a T) -> Self
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        Self {
            data: bytes.as_ref().into(),
            pos: 0,
        }
    }
}

/// Convert from a slice of bytes to an iterator that yields
/// RTCP packet views
impl<'a> From<&'a [u8]> for RTCPPacketViewIterator<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self::new(data)
    }
}

impl<'a> RTCPPacketViewIterator<'a> {
    fn new_impl(&self) -> Result<(usize, RTCPPacketView<'a>), error::Error> {
        match RTCPPacketView::try_new(&self.data[self.pos..]) {
            Ok(view) => match view.valid_packet_len() {
                Ok(len) => Ok((len, view)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }
}

/// Iterator implementation
impl<'a> Iterator for RTCPPacketViewIterator<'a> {
    type Item = Result<RTCPPacketView<'a>, error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.data.len() {
            match self.new_impl() {
                Ok((offset, view)) => {
                    self.pos += offset;
                    Some(Ok(view))
                }
                Err(e) => {
                    self.pos = usize::MAX;
                    Some(Err(e))
                }
            }
        } else {
            None
        }
    }
}

mod test {
    use crate::bits::rtcp::RTCPPacketViewIterator;

    use super::RTCPPacketView;

    const RR_WITH_SDES: [u8; 416] = [
        // packet 0
        0xa1, 0xca, 0x00, 0x1d, // sdes 0 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1d, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x00, // sdes 1 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1e, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x6c, 0x00, 0x00, 0x00, 0x00, // sdes 2 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1c, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x00, 0x00, // additional padding ->
        0x00, 0x00, 0x00, 0x04, // packet 1
        0xa1, 0xca, 0x00, 0x1d, // sdes 0 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1d, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x00, // sdes 1 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1e, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x6c, 0x00, 0x00, 0x00, 0x00, // sdes 2 ->
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1c, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x00, 0x00, // additional padding ->
        0x00, 0x00, 0x00, 0x04, // packet 2 ->
        0x80, 0xc8, 0x00, 0x06, 0x1d, 0x56, 0xbc, 0x2e, 0xe6, 0x65, 0xa5, 0x42, 0x31, 0x31, 0x87,
        0x61, 0x88, 0xba, 0x9a, 0x5c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // packet 3 ->
        0x81, 0xc9, 0x00, 0x07, 0xd2, 0xbd, 0x4e, 0x3e, 0x58, 0xf3, 0x3d, 0xea, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x2c, 0xd8, 0x00, 0x00, 0x07, 0x60, 0x86, 0xd9, 0xf5, 0x81, 0x00, 0x00,
        0x00, 0x01, // sdes 4 ->
        0x81, 0xca, 0x00, 0x08, 0x58, 0xf3, 0x3d, 0xea, 0x01, 0x16, 0x41, 0x43, 0x4c, 0x54, 0x50,
        0x20, 0x43, 0x68, 0x61, 0x6e, 0x6e, 0x65, 0x6c, 0x48, 0x61, 0x6e, 0x64, 0x6c, 0x65, 0x20,
        0x33, 0x30, 0x00, 0x00, 0x00, 0x00, // packet 4 ->
        0x81, 0xca, 0x00, 0x07, 0xd2, 0xbd, 0x4e, 0x3e, 0x01, 0x14, 0x75, 0x6e, 0x6b, 0x6e, 0x6f,
        0x77, 0x6e, 0x40, 0x32, 0x30, 0x30, 0x2e, 0x35, 0x37, 0x2e, 0x37, 0x2e, 0x32, 0x30, 0x34,
        0x00, 0x00, // packet 5 ->
        0x83, 0xcc, 0x00, 0x05, 0x5f, 0xfd, 0xb0, 0x3c, 0x52, 0x49, 0x53, 0x54, 0x83, 0xb1, 0xe7,
        0x69, 0x80, 0x27, 0xfa, 0x1a, 0x00, 0x00, 0x00, 0x00, // packet 6 ->
        0x82, 0xcc, 0x00, 0x05, 0x5f, 0xfd, 0xb0, 0x3c, 0x52, 0x49, 0x53, 0x54, 0x83, 0xb1, 0xe7,
        0x69, 0x80, 0x27, 0xfa, 0x1a, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn test() {
        for packet_res in RTCPPacketViewIterator::new(&RR_WITH_SDES) {
            let p = packet_res.unwrap();
            match p.report().unwrap() {
                super::RTCPReportView::SDES(sdes) => {
                    for item in sdes {
                        println!("{:?}", item);
                    }
                }
                super::RTCPReportView::SR(sr) => {
                    println!("{:?}", sr.ntp_timestamp());
                    for report in sr.reception_reports() {
                        println!("{:?}", report);
                    }
                }
                super::RTCPReportView::RR(rr) => {
                    for report in rr.reception_reports() {
                        println!("{:?}", report);
                    }
                }
                super::RTCPReportView::APP(app) => {
                    println!("{:?} - {:?}", app.name(), app.message())
                }
                _ => {}
            }
        }
    }
}
