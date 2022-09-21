use num_traits::Num;

pub trait RationalPrimitive: Num + Copy + PartialOrd + Into<f64> {}

impl<T> RationalPrimitive for T
where
    T: Num + Copy + PartialOrd,
    f64: From<T>,
{
}

pub trait Rational<T>
where
    T: Num,
{
    fn numerator(&self) -> T;
    fn denominator(&self) -> T;
}

pub trait MakeRational<T>: From<(T, T)>
where
    T: Num + Copy,
{
}

pub trait RationalExt<T>: Rational<T> + MakeRational<T>
where
    T: Num + Copy,
{
    fn reciprocal(&self) -> Self {
        Self::from((self.denominator(), self.numerator()))
    }
}

impl<T> Rational<T> for T
where
    T: Num + Copy,
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
    T: Num + Copy,
{
}

impl<T, K> RationalExt<T> for K
where
    K: Rational<T> + MakeRational<T>,
    T: Num + Copy,
{
}
