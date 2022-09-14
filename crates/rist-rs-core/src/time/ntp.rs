#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    sec: u32,
    frac: u32,
}

impl Timestamp {
    const FRAC: f64 = 4294967295.0;

    pub fn new(sec: u32, frac: u32) -> Timestamp {
        Timestamp { sec, frac }
    }

    pub fn seconds(&self) -> u32 {
        self.sec
    }

    pub fn frac(&self) -> u32 {
        self.frac
    }

    pub fn frac_us(&self) -> f64 {
        (self.frac as f64) * 1.0e6 / Self::FRAC
    }

    pub fn frac_ms(&self) -> f64 {
        (self.frac as f64) * 1.0e3 / Self::FRAC
    }
}
