use core::slice::ChunksExact;

pub mod error {

    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        EndOfPacketReached,
        InvalidPacketLen(usize),
    }
}

const MIN_PACKET_LEN: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct ReceiverReportMessageView<'a> {
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for ReceiverReportMessageView<'a> {
    type Error = error::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        ReceiverReportMessageView::try_new(data)
    }
}

impl<'a> ReceiverReportMessageView<'a> {

    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        if data.len() < 4 {
            Err(error::Error::EndOfPacketReached)
        } else if (data.len() - 4) % 24 != 0 {
            Err(error::Error::InvalidPacketLen(data.len()))
        } else {
            Ok(ReceiverReportMessageView { data })
        }
    }

    pub fn receiver_ssrc(&self) -> u32 {
        u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    pub fn reception_reports(
        &self,
    ) -> impl Iterator<Item = super::rx_report::ReceptionReportView<'a>> {
        self.data[4..].chunks_exact(24).map(|slice| {
            let arr: &'a [u8; 24] = slice
                .try_into()
                .expect(crate::internal::INTERNAL_ERR_PRE_VALIDATED);
            super::rx_report::ReceptionReportView::from(arr)
        })
    }
}
