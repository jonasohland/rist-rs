use crate::time::{
    timebase::MediaTimebasePrimitive,
    timestamp::{ConvertibleMediaTimestamp, MediaTimestamp, MediaTimestampPrimitive},
};

pub trait TimedPacket<TimestampPrim, TimebasePrim>
where
    TimestampPrim: MediaTimestampPrimitive,
    TimebasePrim: MediaTimebasePrimitive,
{
    /// Returns the MediaTimestamp of the packet
    fn media_timestamp(&self) -> MediaTimestamp<TimestampPrim>;

    /// Returns a MediaTimestamp that can be converted to a target timebase
    fn convertible_media_timestamp(&self)
        -> ConvertibleMediaTimestamp<TimestampPrim, TimebasePrim>;
}
