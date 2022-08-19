use core::convert::TryFrom;

const HEADER_SIZE: usize = 8;

#[derive(Debug)]
struct Error {}

struct KeepAlivePacket<'a> {
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for KeepAlivePacket<'a> {
    type Error = Error;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 8 {
            Err(Error {})
        } else {
            Ok(KeepAlivePacket { data })
        }
    }
}

macro_rules! msg_flag {
    ($flag_name:tt, $val:expr, $fun_name:tt, $index:expr, $p:tt) => {
        const $flag_name: u8 = $val;
        impl <'a> KeepAlivePacket<'a> {
            $p fn $fun_name(&self) -> bool {
                (self.data[6 + $index] & $flag_name) != 0
            }
        }
    };
    ($flag_name:tt, $val:expr, $fun_name:tt, $index:expr) => {
        const $flag_name: u8 = $val;
        impl <'a> KeepAlivePacket<'a> {
            fn $fun_name(&self) -> bool {
                (self.data[6 + $index] & $flag_name) != 0
            }
        }
    };
}

// flags part 1
msg_flag!(F0_CAP_MORE, 0x80, cap_more, 0, pub);
msg_flag!(F0_CAP_ROUTING, 0x40, cap_routing, 0, pub);
msg_flag!(F0_CAP_BONDING, 0x20, cap_bonding, 0, pub);
msg_flag!(F0_CAP_ADAPTIVE_ENC, 0x10, cap_adaptive_enc, 0, pub);
msg_flag!(F0_CAP_FEC, 0x8, cap_fec, 0, pub);
msg_flag!(F0_CAP_DASH7, 0x4, cap_dash7, 0, pub);
msg_flag!(F0_CAP_LOAD_SHARING, 0x2, cap_load_sharing, 0, pub);
msg_flag!(F0_CAP_NULL_PACKET_DELETION, 0x1, cap_npd, 0, pub);

// flags part 2
msg_flag!(F1_IS_DISCONNECT, 0x80, is_disconnect, 1);
msg_flag!(F1_IS_RECONNECT, 0x40, is_reconnect, 1);
msg_flag!(F1_CAP_REDUCED_OVERHEAD, 0x20, cap_reduced_overhead, 1, pub);
msg_flag!(F1_CAP_JSON_PROCESSING, 0x10, cap_json_processing, 1, pub);
msg_flag!(F1_CAP_PSK_CHANGE, 0x8, cap_psk_change, 1, pub);

enum KeepAliveMessage<'a> {
    KeepAlive(KeepAlivePacket<'a>),
    Reconnect(KeepAlivePacket<'a>),
    Disconnect(KeepAlivePacket<'a>),
}

impl<'a> TryFrom<&'a [u8]> for KeepAliveMessage<'a> {
    type Error = Error;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let packet = KeepAlivePacket::try_from(data)?;
        if packet.is_reconnect() && packet.is_disconnect() {
            Err(Error {})
        } else if packet.is_disconnect() {
            Ok(KeepAliveMessage::Disconnect(packet))
        } else if packet.is_reconnect() {
            Ok(KeepAliveMessage::Reconnect(packet))
        } else {
            Ok(KeepAliveMessage::KeepAlive(packet))
        }
    }
}

mod test {

    use super::*;

    #[test]
    fn test() {
        let data: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        match KeepAliveMessage::try_from(data.as_slice()).unwrap() {
            KeepAliveMessage::KeepAlive(packet) => {}
            KeepAliveMessage::Reconnect(packet) => {}
            KeepAliveMessage::Disconnect(packet) => {}
        }
    }
}
