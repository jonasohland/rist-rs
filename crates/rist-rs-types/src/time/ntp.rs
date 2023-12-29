use core::fmt::Display;
use std::{ops::Add, time::Duration};

use crate::traits::time::clock::Clock;

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
    const EPOCH_DELTA: u64 = 2_208_988_800;

    pub fn new(sec: u32, frac: u32) -> Timestamp {
        Timestamp { sec, frac }
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
            self.sec as u64 - Self::EPOCH_DELTA,
            self.frac_ns() as u32,
        ))
    }
}

#[test]
fn test() {}
