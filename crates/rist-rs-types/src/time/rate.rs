use crate::traits::math::numbers::{Rational, RationalPrimitive};
use core::{cmp::Ordering, fmt::Display};

#[derive(Clone, Copy, Debug)]
pub struct Rate<T>
where
    T: RationalPrimitive,
{
    pub num: T,
    pub den: T,
}

impl<T> From<(T, T)> for Rate<T>
where
    T: RationalPrimitive,
{
    fn from(rat: (T, T)) -> Self {
        Self {
            num: rat.0,
            den: rat.1,
        }
    }
}

impl<T> From<T> for Rate<T>
where
    T: RationalPrimitive,
    T: Rational<T>,
{
    fn from(v: T) -> Self {
        Self::new(v)
    }
}

impl<T> Rate<T>
where
    T: RationalPrimitive,
{
    pub fn new<K>(rational: K) -> Self
    where
        K: Rational<T>,
    {
        Self {
            num: rational.numerator(),
            den: rational.denominator(),
        }
    }

    pub fn rational(num: T, den: T) -> Self {
        Self { num, den }
    }
}

impl<T> Display for Rate<T>
where
    T: RationalPrimitive,
    T: Display,
    f64: From<T>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.2}", self.to_f64())
    }
}

impl<T> PartialEq for Rate<T>
where
    T: RationalPrimitive,
    T: Into<f64>,
{
    fn eq(&self, other: &Self) -> bool {
        if self.denominator() == T::zero() || other.denominator() == T::zero() {
            false
        } else {
            self.to_f64() == other.to_f64()
        }
    }
}

impl<T> Eq for Rate<T>
where
    T: RationalPrimitive,
    T: PartialOrd,
    T: Into<f64>,
{
}

impl<T> PartialOrd for Rate<T>
where
    T: RationalPrimitive,
    T: PartialOrd,
    f64: From<T>,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.num == other.num && self.den == other.den {
            Some(Ordering::Equal)
        } else if self.den == other.den {
            self.num.partial_cmp(&other.num)
        } else {
            self.to_f64().partial_cmp(&other.to_f64())
        }
    }
}

#[allow(unused)]
mod test {
    use super::*;

    #[test]
    fn zero_rate() {
        let rate = Rate::from((1234, 0));
        assert!(rate.to_f64_checked().is_none());
    }

    #[test]
    fn cmp_equal() {
        assert_eq!(Rate::new(25), Rate::from((25, 1)));
    }

    #[test]
    fn cmp_common_den() {
        assert!(Rate::from((60000, 1001)) > Rate::from((30000, 1001)));
    }

    #[test]
    fn cmp_float() {
        assert!(Rate::from((60000, 1001)) < Rate::new(60));
    }
}

impl<T> Rational<T> for Rate<T>
where
    T: RationalPrimitive,
{
    fn numerator(&self) -> T {
        self.num
    }

    fn denominator(&self) -> T {
        self.den
    }
}
