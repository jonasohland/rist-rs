use crate::{
    time::media::timestamp::MediaTimestampPrimitive, traits::math::numbers::RationalPrimitive,
};

use super::{
    seq::{OrderedPacket, SequenceNumber},
    time::TimedPacket,
};

pub trait RistMediaPacket<Seq, TimestampPrim, TimebasePrim>:
    OrderedPacket<Seq> + TimedPacket<TimestampPrim, TimebasePrim>
where
    Seq: SequenceNumber,
    TimestampPrim: MediaTimestampPrimitive,
    TimebasePrim: RationalPrimitive,
{
}
