use core::fmt::Display;

use num_traits::int::PrimInt;
use num_traits::FromPrimitive;

use crate::traits::math::numbers::Rational;

use super::timebase::MediaTimebasePrimitive;
use super::{rate::Rate, timebase::MediaTimebase};

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
    B: MediaTimebasePrimitive,
{
    ts: MediaTimestamp<T>,
    time_base: Rate<B>,
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
    B: MediaTimebasePrimitive,
{
    pub fn new(ts: MediaTimestamp<T>, time_base: impl Rational<B>) -> Self {
        Self {
            ts,
            time_base: Rate::new(time_base.numerator(), time_base.denominator()),
        }
    }

    #[allow(unused)]
    fn to_timebase_unchecked(self, timebase: impl MediaTimebase<B> + Copy) -> Self {
        Self {
            ts: self
                .time_base
                .convert_timestamp_unchecked(self.ts, timebase),
            time_base: Rate::new(timebase.numerator(), timebase.denominator()),
        }
    }

    #[allow(unused)]
    fn to_timebase(self, timebase: impl MediaTimebase<B> + Copy) -> Option<Self> {
        self.time_base
            .convert_timestamp(self.ts, timebase)
            .map(|ts| Self {
                ts,
                time_base: Rate::new(timebase.numerator(), timebase.denominator()),
            })
    }
}
