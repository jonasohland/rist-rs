use crate::time::media::{timebase::MediaTimebasePrimitive, timestamp::MediaTimestampPrimitive};

use super::{
    seq::{OrderedPacket, SequenceNumber},
    time::TimedPacket,
};

pub trait RistMediaPacket<Seq, TimestampPrim, TimebasePrim>:
    OrderedPacket<Seq> + TimedPacket<TimestampPrim, TimebasePrim>
where
    Seq: SequenceNumber,
    TimestampPrim: MediaTimestampPrimitive,
    TimebasePrim: MediaTimebasePrimitive,
    f64: From<TimebasePrim>,
{
}
