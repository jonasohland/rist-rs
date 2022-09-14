pub mod dispatch_enum;

use crate::{
    packet::Packet,
    processor::{traits::ConnectorSendPacket, Connector},
};

macro_rules! processor_tracing_scope {
    ($ty:literal, $name:expr, $fut:expr) => {
        async move {
            async move {
                $fut.instrument(tracing::span!(tracing::Level::ERROR, $ty, ins = $name))
                    .await
            }
            .instrument(tracing::span!(tracing::Level::ERROR, "proc"))
            .await
        }
    };
}

pub(crate) use processor_tracing_scope;

pub fn send_packet_to(seq: &mut Vec<Connector>, packet: Packet) {
    if !seq.is_empty() {
        if seq.len() == 1 {
            seq[0].send_packet(packet);
        } else if seq.len() == 2 {
            let (p1, p2) = packet.dup();
            seq[0].send_packet(p1);
            seq[1].send_packet(p2);
        } else {
            seq.iter_mut()
                .zip(packet.into_iter())
                .for_each(|(input, packet)| input.send_packet(packet));
        }
    } else {
        tracing::warn!(
            "drop packet from {} because output is not connected",
            packet.source_addr
        );
    }
}
