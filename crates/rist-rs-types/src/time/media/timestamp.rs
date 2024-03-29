use core::fmt::Display;

use num_traits::FromPrimitive;
use num_traits::PrimInt;

use crate::time::rate::Rate;
use crate::traits::math::numbers::Rational;
use crate::traits::math::numbers::RationalPrimitive;

use super::timebase::MediaTimebase;

pub trait MediaTimestampPrimitive: PrimInt + Into<f64> {}

impl<T> MediaTimestampPrimitive for T
where
    T: PrimInt,
    f64: From<T>,
{
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaTimestamp<T>(T)
where
    T: MediaTimestampPrimitive;

impl<T> Display for MediaTimestamp<T>
where
    T: MediaTimestampPrimitive,
    T: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConvertibleMediaTimestamp<T, B>
where
    T: MediaTimestampPrimitive,
    B: RationalPrimitive,
{
    ts: MediaTimestamp<T>,
    timebase: Rate<B>,
    framerate: Rate<B>,
}

impl<T> MediaTimestamp<T>
where
    T: MediaTimestampPrimitive,
{
    /// Make a new timestamp
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn value(self) -> T {
        self.0
    }
}

impl<T, B> ConvertibleMediaTimestamp<T, B>
where
    T: MediaTimestampPrimitive + FromPrimitive,
    B: RationalPrimitive,
{
    pub fn new(
        ts: MediaTimestamp<T>,
        timebase: impl Rational<B>,
        framerate: impl Rational<B>,
    ) -> Self {
        Self {
            ts,
            timebase: Rate::new(timebase),
            framerate: Rate::new(framerate),
        }
    }

    #[allow(unused)]
    fn to_timebase_unchecked(self, timebase: impl MediaTimebase<B> + Copy) -> Self {
        Self {
            ts: self.timebase.convert_timestamp_unchecked(self.ts, timebase),
            timebase: Rate::new(timebase),
            framerate: self.framerate,
        }
    }

    #[allow(unused)]
    fn to_timebase(self, timebase: impl MediaTimebase<B> + Copy) -> Option<Self> {
        self.timebase
            .convert_timestamp(self.ts, timebase)
            .map(|ts| Self {
                ts,
                timebase: Rate::new(timebase),
                framerate: self.framerate,
            })
    }
}
