use num_traits::PrimInt;

pub trait Rational<T>
where
    T: PrimInt,
{
    fn numerator(&self) -> T;
    fn denominator(&self) -> T;
}

pub trait MakeRational<T>: From<(T, T)>
where
    T: PrimInt + Copy,
{
}

pub trait RationalExt<T>: Rational<T> + MakeRational<T>
where
    T: PrimInt + Copy,
{
    fn reciprocal(&self) -> Self {
        Self::from((self.denominator(), self.numerator()))
    }
}

impl<T> Rational<T> for T
where
    T: PrimInt,
{
    fn numerator(&self) -> T {
        *self
    }

    fn denominator(&self) -> T {
        T::one()
    }
}

impl<T, K> MakeRational<T> for K
where
    K: Rational<T> + From<(T, T)>,
    T: PrimInt,
{
}

impl<T, K> RationalExt<T> for K
where
    K: Rational<T> + MakeRational<T>,
    T: PrimInt,
{
}
