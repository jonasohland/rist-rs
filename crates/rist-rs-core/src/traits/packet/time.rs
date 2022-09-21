use crate::{
    time::media::timestamp::{ConvertibleMediaTimestamp, MediaTimestamp, MediaTimestampPrimitive},
    traits::math::numbers::RationalPrimitive,
};

pub trait TimedPacket<TimestampPrim, TimebasePrim>
where
    TimestampPrim: MediaTimestampPrimitive,
    TimebasePrim: RationalPrimitive,
{
    /// Returns the MediaTimestamp of the packet
    fn media_timestamp(&self) -> MediaTimestamp<TimestampPrim>;

    /// Returns a MediaTimestamp that can be converted to a target timebase
    fn convertible_media_timestamp(&self)
        -> ConvertibleMediaTimestamp<TimestampPrim, TimebasePrim>;
}
