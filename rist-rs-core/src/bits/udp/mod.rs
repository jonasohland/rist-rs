mod reduced;

pub trait UDPPacket {
    fn source_port(&self) -> u16;
    fn destination_port(&self) -> u16;
}
