use crate::{
    time::{timebase::MediaTimebasePrimitive, timestamp::MediaTimestampPrimitive},
    traits::{
        packet::{rist::RistMediaPacket, seq::SequenceNumber},
        time::clock::Clock,
    },
};

pub enum EnqueueResult<P> {
    TooLate(P),
}

pub enum Event<P> {
    Packet(P),
}

pub trait RistMediaPacketBuffer<Clk, Packet, Seq, TimestampPrim, TimebasePrim>
where
    Seq: SequenceNumber,
    Packet: RistMediaPacket<Seq, TimestampPrim, TimebasePrim>,
    Clk: Clock,
    TimestampPrim: MediaTimestampPrimitive,
    TimebasePrim: MediaTimebasePrimitive,
    f64: From<TimebasePrim>,
{
    /// Enqueue a packet
    fn enqueue(&mut self, packet: Packet) -> EnqueueResult<Packet>;

    /// Get the next event from the buffer
    fn next_event(&mut self, now: Clk::TimePoint) -> Option<Event<Packet>>;

    /// Estimate the time at which the next event will be available
    fn next_wake(&self) -> Option<Clk::TimePoint>;
}
