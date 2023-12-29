use core::fmt::Display;
use std::{ops::Add, time::Duration};

use crate::traits::time::clock::{Clock, TimePoint};

#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    sec: u32,
    frac: u32,
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sec)
    }
}

impl Timestamp {
    const FRAC: f64 = std::u32::MAX as f64;

    pub fn new(sec: u32, frac: u32) -> Timestamp {
        Timestamp { sec, frac }
    }

    pub fn from_time_point<C: Clock>(
        tp: C::TimePoint,
    ) -> Result<Timestamp, <C::TimePoint as TimePoint>::Error> {
        let ep = tp.duration_since(C::epoch())?;
        let ns = (ep.subsec_nanos() as f64 * Self::FRAC * 1.0e-9) as u32;
        let s = (ep.as_secs() as i64 + C::ntp_epoch_offset()) as u32;
        Ok(Self::new(s, ns))
    }

    pub fn seconds(&self) -> u32 {
        self.sec
    }

    pub fn frac(&self) -> u32 {
        self.frac
    }

    pub fn frac_ns(&self) -> f64 {
        (self.frac as f64) * 1.0e9 / Self::FRAC
    }

    pub fn frac_us(&self) -> f64 {
        (self.frac as f64) * Self::FRAC / 1.0e6
    }

    pub fn frac_ms(&self) -> f64 {
        (self.frac as f64) * Self::FRAC / 1.0e3
    }

    pub fn to_instant<C: Clock>(&self) -> C::TimePoint {
        C::epoch().add(Duration::new(
            (self.sec as i64 - C::ntp_epoch_offset()) as u64,
            self.frac_ns() as u32,
        ))
    }
}

#[test]
fn test() {}
