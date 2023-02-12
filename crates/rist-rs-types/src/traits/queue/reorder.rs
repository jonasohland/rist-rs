use crate::traits::packet::seq::{OrderedPacket, SequenceNumber};

/// Output of a reordering buffer
#[derive(Debug)]
pub enum ReorderQueueEvent<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    /// The next packet in the sequence
    Packet(P),

    /// The next packet in the sequence has not arrived yet,
    /// need more packets
    NeedMore,

    /// The current packet is considered missing by the buffer
    /// and the packet for the next sequence number will be returned
    /// in the next iteration
    Missing,

    /// The sequence number of incoming packet was reset. The next packet will have the
    /// sequence number returned in the variant
    Reset(S),
}

/**
 * Write unordered packets
 */
pub trait ReorderQueueInput<S: SequenceNumber, P: OrderedPacket<S>> {
    fn put(&mut self, packet: P) -> Option<P>;
}

/**
 * Read reordered packets
 */
pub trait ReorderQueueOutput<S: SequenceNumber, P: OrderedPacket<S>> {
    fn next_event(&mut self) -> ReorderQueueEvent<S, P>;
}
