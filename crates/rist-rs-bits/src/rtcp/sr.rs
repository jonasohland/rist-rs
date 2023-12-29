use crate::util::read_int;

pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        InvalidPacketLen(usize),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SenderReportMessageView<'a> {
    data: &'a [u8],
}

const MIN_PACKET_LEN: usize = 24;

impl<'a> TryFrom<&'a [u8]> for SenderReportMessageView<'a> {
    type Error = error::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        SenderReportMessageView::try_new(data)
    }
}

impl<'a> SenderReportMessageView<'a> {
    const NTP_MSB_OFFSET: usize = 4;
    const NTP_LSB_OFFSET: usize = Self::NTP_MSB_OFFSET + 4;
    const RTP_TS_OFFSET: usize = Self::NTP_LSB_OFFSET + 4;
    const PACKET_COUNT_OFFSET: usize = Self::RTP_TS_OFFSET + 4;
    const OCTET_COUNT_OFFSET: usize = Self::PACKET_COUNT_OFFSET + 4;
    const RX_REPORTS_OFFSET: usize = Self::OCTET_COUNT_OFFSET + 4;

    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() < MIN_PACKET_LEN || (data.len() % 24) != 0 {
            Err(error::Error::InvalidPacketLen(data.len()))
        } else {
            Ok(SenderReportMessageView { data })
        }
    }

    pub fn ntp_timestamp(&self) -> rist_rs_types::time::ntp::Timestamp {
        rist_rs_types::time::ntp::Timestamp::new(
            read_int!(self.data, u32, Self::NTP_MSB_OFFSET),
            read_int!(self.data, u32, Self::NTP_LSB_OFFSET),
        )
    }

    pub fn rtp_timestamp(&self) -> u32 {
        read_int!(self.data, u32, Self::RTP_TS_OFFSET)
    }

    pub fn packet_count(&self) -> u32 {
        read_int!(self.data, u32, Self::PACKET_COUNT_OFFSET)
    }

    pub fn octet_count(&self) -> u32 {
        read_int!(self.data, u32, Self::OCTET_COUNT_OFFSET)
    }

    pub fn reception_reports(
        &self,
    ) -> impl Iterator<Item = super::rx_report::ReceptionReportView<'a>> {
        self.data[Self::RX_REPORTS_OFFSET..]
            .chunks_exact(24)
            .map(|slice| {
                let arr: &'a [u8; 24] = slice
                    .try_into()
                    .expect(rist_rs_types::internal::INTERNAL_ERR_PRE_VALIDATED);
                super::rx_report::ReceptionReportView::from(arr)
            })
    }
}
