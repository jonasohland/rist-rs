use core::{
    fmt::Debug,
    hash::Hash,
    ops::{Add, AddAssign, Sub},
    time::Duration,
};
use rist_rs_macros::cfg_std;

pub mod derived_clock;

///
pub trait TimePoint:
    Sized
    + Clone
    + Copy
    + Add<Duration>
    + AddAssign<Duration>
    + Sub<Duration>
    + Debug
    + Eq
    + Hash
    + Ord
    + PartialEq<Self>
    + PartialOrd<Self>
{
    type Error;

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
    fn saturating_duration_since(&self, earlier: Self) -> Result<Duration, Self::Error>;

    /// Returns Some(t) where t is the time self + duration if t can be represented
    /// as SystemTime (which means it’s inside the bounds of the underlying data structure), None otherwise.
    fn checked_add(&self, duration: Duration) -> Option<Self>;

    /// Returns Some(t) where t is the time self - duration if t can be represented
    /// as SystemTime (which means it’s inside the bounds of the underlying data structure), None otherwise.
    fn checked_sub(&self, duration: Duration) -> Option<Self>;
}

pub trait Clock {
    type TimePoint: TimePoint;

    /// Returns the current time
    fn now() -> Self::TimePoint;

    /// Returns true if the clocks time is monotonically increasing
    fn is_monotonic() -> bool;
}

cfg_std! {

    use std::time::{Instant, SystemTime, SystemTimeError};

    /// A monotonic clock, available with the std feature enabled
    struct StdMonotonicClock;

    impl Clock for StdMonotonicClock {

        type TimePoint = Instant;

        fn now() -> Self::TimePoint {
            Instant::now()
        }

        fn is_monotonic() -> bool {
            true
        }

    }

    impl TimePoint for std::time::Instant {

        type Error = std::convert::Infallible;

        fn duration_since(&self, earlier: Self) -> Result<Duration, Self::Error> {
            Ok(self.duration_since(earlier))
        }

        fn saturating_duration_since(&self, earlier: Self) -> Result<Duration, Self::Error> {
            Ok(self.saturating_duration_since(earlier))
        }

        fn checked_add(&self, duration: Duration) -> Option<Self> {
            self.checked_add(duration)
        }

        fn checked_sub(&self, duration: Duration) -> Option<Self> {
            self.checked_sub(duration)
        }
    }

    /// System clock, not steady, might drift or jump
    struct StdSystemClock;

    impl Clock for StdSystemClock {

        type TimePoint = SystemTime;

        fn now() -> Self::TimePoint {
            SystemTime::now()
        }

        fn is_monotonic() -> bool {
            false
        }
    }

    impl TimePoint for SystemTime {

        type Error = SystemTimeError;

        fn duration_since(&self, earlier: Self) -> Result<Duration, Self::Error> {
            self.duration_since(earlier)
        }

        fn saturating_duration_since(&self, _earlier: Self) -> Result<Duration, Self::Error> {
            todo!()
        }

        fn checked_add(&self, duration: Duration) -> Option<Self> {
            self.checked_add(duration)
        }

        fn checked_sub(&self, duration: Duration) -> Option<Self> {
            self.checked_sub(duration)
        }
    }


}
