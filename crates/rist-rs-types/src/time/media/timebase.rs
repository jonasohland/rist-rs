use super::{
    framerate::MediaFramerate,
    timestamp::{MediaTimestamp, MediaTimestampPrimitive},
};
use crate::traits::math::numbers::{Rational, RationalExt, RationalPrimitive};
use num_traits::FromPrimitive;

pub trait MediaTimebase<T>: Rational<T>
where
    T: RationalPrimitive,
{
    fn make_framerate<Rep, K>(&self) -> Rep
    where
        K: RationalPrimitive,
        Rep: MediaFramerate<K> + RationalExt<T>,
    {
        Rep::from((self.numerator(), self.denominator())).reciprocal()
    }

    fn can_convert(&self, to: impl MediaTimebase<T>) -> bool {
        if self.denominator() > to.denominator() {
            self.denominator() % to.denominator() == T::zero()
        } else {
            to.denominator() % self.denominator() == T::zero()
        }
    }

    fn convert_timestamp_unchecked<K>(
        &self,
        ts: MediaTimestamp<K>,
        target: impl MediaTimebase<T>,
    ) -> MediaTimestamp<K>
    where
        K: MediaTimestampPrimitive + FromPrimitive,
    {
        MediaTimestamp::new(
            K::from_f64(
                ts.value().into() * (target.denominator() * self.numerator()).into()
                    / (target.numerator() * self.denominator()).into(),
            )
            .unwrap_or_else(K::zero),
        )
    }

    fn convert_timestamp<K>(
        &self,
        ts: MediaTimestamp<K>,
        target: impl MediaTimebase<T> + Copy,
    ) -> Option<MediaTimestamp<K>>
    where
        K: MediaTimestampPrimitive + FromPrimitive,
    {
        self.can_convert(target)
            .then(|| {
                K::from_f64(
                    ts.value().into() * (target.denominator() * self.numerator()).into()
                        / (target.numerator() * self.denominator()).into(),
                )
                .map(|ts| MediaTimestamp::new(ts))
            })
            .flatten()
    }
}

pub trait MediaTimebaseExt<T>: MediaTimebase<T> + RationalExt<T>
where
    T: RationalPrimitive,
{
    fn to_framerate(&self) -> Self {
        self.reciprocal()
    }
}

impl<T, K> MediaTimebase<T> for K
where
    T: RationalPrimitive,
    K: Rational<T>,
{
}

impl<T, K> MediaTimebaseExt<T> for K
where
    T: RationalPrimitive,
    K: RationalExt<T>,
{
}

#[allow(unused)]
mod test {

    use super::*;
    use crate::time::rate::Rate;

    #[test]
    fn test_make_framerate() {
        assert_eq!(Rate::rational(25., 1.), 0.04.make_framerate());
    }

    #[test]
    fn test_to_framerate() {
        assert_eq!(Rate::rational(25, 1), Rate::rational(1, 25).to_framerate());
    }
}
