use super::timebase::MediaTimebase;
use crate::traits::math::numbers::{Rational, RationalExt, RationalPrimitive};

pub trait MediaFramerate<T>: Rational<T> + Sized
where
    T: RationalPrimitive,
{
    fn make_timebase<Rep, K>(&self) -> Rep
    where
        K: RationalPrimitive + PartialOrd,
        Rep: MediaTimebase<K> + RationalExt<T>,
    {
        Rep::from((self.numerator(), self.denominator())).reciprocal()
    }
}

impl<T, K> MediaFramerate<T> for K
where
    T: RationalPrimitive,
    K: Rational<T>,
{
}

impl<T, K> MediaFramerateExt<T> for K
where
    T: RationalPrimitive,
    K: RationalExt<T>,
{
}

pub trait MediaFramerateExt<T>: MediaFramerate<T> + RationalExt<T>
where
    T: RationalPrimitive,
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
        assert_eq!(Rate::new(0.04), 25.0.make_timebase());
    }

    #[test]
    fn test_to_timebase() {
        assert_eq!(Rate::rational(1, 25), Rate::rational(25, 1).to_timebase());
    }
}
