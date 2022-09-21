use super::timebase::{MediaTimebase, MediaTimebasePrimitive};
use crate::traits::math::numbers::{Rational, RationalExt};
use num_traits::PrimInt;

pub trait MediaFrameratePrimitive: PrimInt + Into<f64> {}

impl<T> MediaFrameratePrimitive for T
where
    T: PrimInt,
    f64: From<T>,
{
}

pub trait MediaFramerate<T>: Rational<T> + Sized
where
    T: MediaFrameratePrimitive,
{
    fn make_timebase<Rep, K>(&self) -> Rep
    where
        K: MediaTimebasePrimitive,
        Rep: MediaTimebase<K> + RationalExt<T>,
    {
        Rep::from((self.numerator(), self.denominator())).reciprocal()
    }
}

impl<T, K> MediaFramerate<T> for K
where
    T: MediaFrameratePrimitive,
    K: Rational<T>,
{
}

impl<T, K> MediaFramerateExt<T> for K
where
    T: MediaFrameratePrimitive,
    K: RationalExt<T>,
{
}

pub trait MediaFramerateExt<T>: MediaFramerate<T> + RationalExt<T>
where
    T: MediaFrameratePrimitive + Copy,
{
    fn to_timebase(self) -> Self {
        self.reciprocal()
    }
}

#[allow(unused)]
mod test {

    use super::*;
    use crate::time::rate::Rate;

    #[test]
    fn test_make_timebase() {
        assert_eq!(Rate::new(1, 25), 25.make_timebase());
    }

    #[test]
    fn test_to_timebase() {
        assert_eq!(Rate::new(1, 25), Rate::new(25, 1).to_timebase());
    }
}
