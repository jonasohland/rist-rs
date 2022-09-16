use core::time::Duration;

pub trait DerivedClock {
    /// Get the duration that has passed since the clock epoch at the current moment in time
    fn duration_since_epoch(&self) -> Duration;

    /// Adjust the internal clock speed to match the speed of a remote clock
    fn adjust_from_remote(&mut self, remote_time_since_epoch: Duration, rtt: Duration);
}
