use crate::traits::math::numbers::Rational;
use core::{cmp::Ordering, fmt::Display};
use num_traits::{float::FloatCore, Num};

#[derive(Clone, Copy, Debug)]
pub struct Rate<T>
where
    T: Num + Copy,
{
    pub num: T,
    pub den: T,
}

impl<T> From<(T, T)> for Rate<T>
where
    T: Num + Copy,
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
    T: Num + Copy,
    T: Rational<T>,
{
    fn from(v: T) -> Self {
        Self::new(v)
    }
}

impl<T> Rate<T>
where
    T: Num + Copy,
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
    T: Num + Copy,
    T: Display,
    f64: From<T>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.2}", self.as_float::<f64>())
    }
}

impl<T> Rate<T>
where
    T: Num + Copy,
{
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

impl<T> PartialEq for Rate<T>
where
    T: Num + Copy,
    T: PartialOrd,
    T: Into<f64>,
{
    fn eq(&self, other: &Self) -> bool {
        if self.denominator() == T::zero() || other.denominator() == T::zero() {
            false
        } else {
            (self.num.into() / self.den.into()) == (other.num.into() / other.den.into())
        }
    }
}

impl<T> Eq for Rate<T>
where
    T: Num + Copy,
    T: PartialOrd,
    T: Into<f64>,
{
}

impl<T> PartialOrd for Rate<T>
where
    T: Num + Copy,
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
        let rate = Rate::from((1234, 0));
        assert_eq!(rate.as_float::<f64>(), 0.);
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
    T: Num + Copy,
{
    fn numerator(&self) -> T {
        self.num
    }

    fn denominator(&self) -> T {
        self.den
    }
}
