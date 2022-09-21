use crate::traits::math::numbers::Rational;
use core::{cmp::Ordering, fmt::Display};
use num_traits::{float::FloatCore, PrimInt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate<T>
where
    T: PrimInt,
{
    pub num: T,
    pub den: T,
}

impl<T> From<(T, T)> for Rate<T>
where
    T: PrimInt,
{
    fn from(rat: (T, T)) -> Self {
        Self {
            num: rat.0,
            den: rat.1,
        }
    }
}

impl<T> Display for Rate<T>
where
    T: PrimInt,
    T: Display,
    f64: From<T>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.2}", self.as_float::<f64>())
    }
}

impl<T> Rate<T>
where
    T: PrimInt,
{
    pub fn new(num: T, den: T) -> Self {
        Self { num, den }
    }

    pub fn as_float<K: FloatCore>(&self) -> K
    where
        K: From<T>,
    {
        if self.den == T::zero() {
            K::zero()
        } else {
            <K as From<T>>::from(self.num) / <K as From<T>>::from(self.den)
        }
    }
}

impl<T> PartialOrd for Rate<T>
where
    T: PrimInt,
    T: PartialOrd,
    f64: From<T>,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.num == other.num && self.den == other.den {
            Some(Ordering::Equal)
        } else if self.den == other.den {
            self.num.partial_cmp(&other.num)
        } else {
            self.as_float::<f64>().partial_cmp(&other.as_float::<f64>())
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

impl<T> Rational<T> for Rate<T>
where
    T: PrimInt + Copy,
{
    fn numerator(&self) -> T {
        self.num
    }

    fn denominator(&self) -> T {
        self.den
    }
}

impl<T> From<T> for Rate<T>
where
    T: PrimInt + Copy,
    T: Rational<T>,
{
    fn from(v: T) -> Self {
        Self::new(v.numerator(), v.denominator())
    }
}
