#[derive(Debug, Clone, Copy)]
pub struct ReceptionReportView<'a> {
    data: &'a [u8; 24],
}

impl<'a> From<&'a [u8; 24]> for ReceptionReportView<'a> {
    fn from(data: &'a [u8; 24]) -> Self {
        Self { data }
    }
}

impl<'a> ReceptionReportView<'a> {
    // TODO: Implement accessors!
}
