use core::fmt::{Debug, Display};
use num_traits::{Bounded, NumOps, One, Unsigned, WrappingAdd, WrappingSub, Zero};

pub mod ring;

pub trait SequenceNumber:
    Unsigned
    + Bounded
    + NumOps
    + Zero
    + One
    + Ord
    + Eq
    + Display
    + Debug
    + Into<u64>
    + Copy
    + WrappingAdd
    + WrappingSub
{
}

impl SequenceNumber for u16 {}

impl SequenceNumber for u32 {}

/**
 * A packet from which a sequence number can be read
 */
pub trait OrderedPacket<S: SequenceNumber> {
    fn sequence_number(&self) -> S;
}

/**
 * Write unordered packets
 */
pub trait ReorderWriter<S: SequenceNumber, P: OrderedPacket<S>> {
    fn send(&mut self, packet: P) -> Option<P>;
}

/// Output of a reordering buffer
#[derive(Debug)]
pub enum ReorderOutput<S, P>
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
 * Read reordered packets
 */
pub trait ReorderReader<S: SequenceNumber, P: OrderedPacket<S>> {
    fn receive(&mut self) -> ReorderOutput<S, P>;
}

