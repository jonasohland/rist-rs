use core::{cmp::Ordering, fmt::Display};

use num_traits::float::FloatCore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rate {
    pub num: u32,
    pub den: u32,
}

impl Display for Rate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.2}", self.as_float::<f64>())
    }
}

impl Rate {
    fn new(num: u32, den: u32) -> Self {
        Self { num, den }
    }

    fn as_float<T: FloatCore>(&self) -> T
    where
        T: From<u32>,
    {
        if self.den == 0 {
            T::zero()
        } else {
            <T as From<u32>>::from(self.num) / <T as From<u32>>::from(self.den)
        }
    }
}

impl PartialOrd for Rate {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.num == other.num && self.den == other.den {
            Some(Ordering::Equal)
        } else if self.den == other.den {
            self.num.partial_cmp(&other.num)
        } else {
            self.as_float::<f64>().partial_cmp(&other.as_float())
        }
    }
}

#[allow(unused)]
mod test {
    use super::*;

    #[test]
    fn zero_rate() {
        let rate = Rate::new(1234, 0);
        assert_eq!(rate.as_float::<f64>(), 0.);
    }

    #[test]
    fn cmp_equal() {
        assert_eq!(Rate::new(25, 1), Rate::new(25, 1));
    }

    #[test]
    fn cmp_common_den() {
        assert!(Rate::new(60000, 1001) > Rate::new(30000, 1001));
    }

    #[test]
    fn cmp_float() {
        assert!(Rate::new(60000, 1001) < Rate::new(60, 1));
    }
}
