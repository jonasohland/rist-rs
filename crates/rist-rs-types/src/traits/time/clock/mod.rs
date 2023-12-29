use core::{
    fmt::Debug,
    hash::Hash,
    ops::{Add, AddAssign, Sub},
    time::Duration,
};
use rist_rs_macros::cfg_std;
use std::{fmt::Display, time::UNIX_EPOCH};

pub mod derived_clock;

///
pub trait TimePoint:
    Sized
    + Clone
    + Copy
    + Send
    + Add<Duration, Output = Self>
    + AddAssign<Duration>
    + Sub<Duration>
    + Debug
    + Eq
    + Hash
    + Ord
    + PartialEq<Self>
    + PartialOrd<Self>
{
    type Error: Debug + Display + Send + 'static;

    /// Returns the amount of time elapsed from an earlier point in time.
    /// This function may fail because measurements taken earlier are not guaranteed
    /// to always be before later measurements (due to anomalies such as the
    /// system clock being adjusted either forwards or backwards). Instant can be used
    /// to measure elapsed time without this risk of failure.
    ///
    /// If successful, Ok(Duration) is returned where the duration represents
    /// the amount of time elapsed from the specified measurement to this one.
    ///
    /// Returns an Err if earlier is later than self, and the error contains how far from self the time is.
    fn duration_since(&self, earlier: Self) -> Result<Duration, Self::Error>;

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    fn saturating_duration_since(&self, earlier: Self) -> Duration;

    /// Returns Some(t) where t is the time self + duration if t can be represented
    /// as SystemTime (which means it’s inside the bounds of the underlying data structure), None otherwise.
    fn checked_add(&self, duration: Duration) -> Option<Self>;

    /// Returns Some(t) where t is the time self - duration if t can be represented
    /// as SystemTime (which means it’s inside the bounds of the underlying data structure), None otherwise.
    fn checked_sub(&self, duration: Duration) -> Option<Self>;
}

pub trait Clock: Clone + Send + 'static {
    type TimePoint: TimePoint;

    /// Returns a time point that is now, or guaranteed to be before the current time
    fn immediate(&self) -> Self::TimePoint;

    /// Returns the current time
    fn now(&self) -> Self::TimePoint;

    /// Returns true if the clocks time is monotonically increasing
    fn is_monotonic(&self) -> bool;

    // This clocks epoch as a point in time
    fn epoch() -> Self::TimePoint;

    // Offset of this clocks epoch to the NTP epoch in seconds
    fn ntp_epoch_offset() -> i64;
}

cfg_std! {

    use std::time::{SystemTime, SystemTimeError};

    /// System clock, not steady, might drift or jump
    #[derive(Clone)]
    pub struct StdSystemClock;

    impl Clock for StdSystemClock {

        type TimePoint = SystemTime;

        fn immediate(&self) -> Self::TimePoint {
            UNIX_EPOCH
        }

        fn now(&self) -> Self::TimePoint {
            SystemTime::now()
        }

        fn is_monotonic(&self) -> bool {
            false
        }

        fn epoch() -> Self::TimePoint {
            UNIX_EPOCH
        }

        fn ntp_epoch_offset() -> i64 {
            2208988800
        }
    }

    impl TimePoint for SystemTime {

        type Error = SystemTimeError;

        fn duration_since(&self, earlier: Self) -> Result<Duration, Self::Error> {
            self.duration_since(earlier)
        }

        fn saturating_duration_since(&self, earlier: Self) -> Duration {
            self.duration_since(earlier).unwrap_or_else(|_| (Duration::from_secs(0)))
        }

        fn checked_add(&self, duration: Duration) -> Option<Self> {
            self.checked_add(duration)
        }

        fn checked_sub(&self, duration: Duration) -> Option<Self> {
            self.checked_sub(duration)
        }
    }

}
