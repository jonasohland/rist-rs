use core::time::Duration;

use super::Clock;

pub trait DerivedClock: Clock {
    /// Adjust the internal clock speed to match the speed of a remote clock
    fn adjust_from_remote(&mut self, remote_time_since_epoch: Duration, rtt: Duration);
}
