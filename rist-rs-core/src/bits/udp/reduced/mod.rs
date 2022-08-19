use crate::bits::util;

use super::UDPPacket;

struct UDPReducedHeaderPacket<'a> {
    data: &'a [u8],
}

impl<'a> UDPPacket for UDPReducedHeaderPacket<'a> {
    fn source_port(&self) -> u16 {
        util::read_int!(self.data, u16, 0)
    }

    fn destination_port(&self) -> u16 {
        util::read_int!(self.data, u16, 2)
    }
}

mod test {

    #[test]
    fn test() {
        use log::log;
    }
}
