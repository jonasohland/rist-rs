use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::time::Instant;

pub struct CreditCounter {
    credit: f64,
    cps: f64,
    size: f64,
    last_credit_update: Instant,
}

impl CreditCounter {
    pub fn new(credit: u64, size: u64) -> Self {
        Self {
            credit: 0.,
            cps: credit as f64,
            size: size as f64,
            last_credit_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        if self.credit < self.size {
            let now = Instant::now();
            self.credit = (self.credit + (now - self.last_credit_update).as_secs_f64() * self.cps)
                .min(self.size);
            self.last_credit_update = now;
        }
    }

    pub fn take(&mut self, count: u64) -> Option<u64> {
        if self.credit.floor() as u64 >= count {
            self.credit -= count as f64;
            Some(count)
        } else {
            None
        }
    }

    pub fn sleep_time_to_availability(&self, count: u64) -> Result<Duration> {
        match count as f64 {
            c if c > self.cps => Err(anyhow!("")),
            c if c <= self.credit => Ok(Duration::from_secs(0)),
            c => Ok(Duration::from_secs_f64((c - self.credit) / self.cps)),
        }
    }
}
