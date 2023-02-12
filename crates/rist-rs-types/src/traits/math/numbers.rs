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
    T: RationalPrimitive,
{
    fn numerator(&self) -> T;
    fn denominator(&self) -> T;

    fn to_f64(&self) -> f64 {
        self.numerator().into() / self.denominator().into()
    }

    fn to_f64_checked(&self) -> Option<f64> {
        let n = self.to_f64();
        n.is_finite().then_some(n)
    }
}

pub trait MakeRational<T>: From<(T, T)>
where
    T: RationalPrimitive,
{
}

pub trait RationalExt<T>: Rational<T> + MakeRational<T>
where
    T: RationalPrimitive,
{
    fn reciprocal(&self) -> Self {
        Self::from((self.denominator(), self.numerator()))
    }
}

impl<T> Rational<T> for T
where
    T: RationalPrimitive,
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
    T: RationalPrimitive,
{
}

impl<T, K> RationalExt<T> for K
where
    K: Rational<T> + MakeRational<T>,
    T: RationalPrimitive,
{
}
