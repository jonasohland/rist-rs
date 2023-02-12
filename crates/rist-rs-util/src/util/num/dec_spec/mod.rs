use alloc::string::String;
use core::convert::TryFrom;
use core::fmt::{Debug, Display};
use core::ops::Mul;
use core::str::FromStr;
use rist_rs_macros::cfg_serde;

cfg_serde!(
    pub mod ser_de;
);

#[derive(Debug)]
pub enum FromDecSpecError {
    ParseError(&'static str),
    ConvertNumError,
    ParseNumError,
}

impl Display for FromDecSpecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FromDecSpecError::ParseError(e) => write!(f, "{e}"),
            FromDecSpecError::ConvertNumError => write!(f, "could not convert to target type"),
            FromDecSpecError::ParseNumError => write!(f, "failed to parse number part"),
        }
    }
}

pub trait FromDecSpecFloat<T>: Sized {
    fn from_f64(spec: f64) -> Self;

    fn from_dec_spec(spec: &str) -> Result<Self, FromDecSpecError> {
        let mut char_iter = spec.chars().rev();
        match match char_iter.next() {
            Some(c) => match c {
                'y' => Ok(Some(0.000_000_000_000_000_000_000_001)),
                'z' => Ok(Some(0.000_000_000_000_000_000_001)),
                'a' => Ok(Some(0.000_000_000_000_000_001)),
                'f' => Ok(Some(0.000_000_000_000_001)),
                'p' => Ok(Some(0.000_000_000_001)),
                'n' => Ok(Some(0.000_000_001)),
                'u' | 'μ' => Ok(Some(0.000_001)),
                'm' => Ok(Some(0.001)),
                'c' => Ok(Some(0.01)),
                'd' => Ok(Some(0.1)),
                'D' => Ok(Some(10.)),
                'H' => Ok(Some(100.)),
                'K' => Ok(Some(1000.)),
                'M' => Ok(Some(1_000_000.)),
                'G' => Ok(Some(1_000_000_000.)),
                'T' => Ok(Some(1_000_000_000_000.)),
                'P' => Ok(Some(1_000_000_000_000_000.)),
                'E' => Ok(Some(1_000_000_000_000_000_000.)),
                'Z' => Ok(Some(1_000_000_000_000_000_000_000.)),
                'Y' => Ok(Some(1_000_000_000_000_000_000_000_000.)),
                a if a.is_numeric() => Ok(None),
                _ => Err(FromDecSpecError::ParseError("invalid dec char")),
            },
            None => Err(FromDecSpecError::ParseError("empty")),
        }? {
            Some(num) => {
                let num_str = char_iter.rev().collect::<String>();
                Ok(Self::from_f64(
                    f64::from_str(num_str.as_str()).map_err(|_| FromDecSpecError::ParseNumError)?
                        * num,
                ))
            }
            None => Ok(Self::from_f64(
                f64::from_str(spec).map_err(|_| FromDecSpecError::ParseNumError)?,
            )),
        }
    }
}

pub trait FromDecSpecInt<T>: TryFrom<i128> + Mul<Self, Output = Self>
where
    T: TryFrom<i128>,
{
    fn from_dec_spec(spec: &str) -> Result<T, FromDecSpecError>
    where
        T: Mul<T, Output = T>,
    {
        T::try_from(f64::from_dec_spec(spec)? as i128)
            .map_err(|_| FromDecSpecError::ConvertNumError)
    }
}

impl FromDecSpecFloat<f64> for f64 {
    fn from_f64(spec: f64) -> Self {
        spec
    }
}

impl FromDecSpecFloat<f32> for f32 {
    fn from_f64(spec: f64) -> Self {
        spec as f32
    }
}

impl FromDecSpecInt<u8> for u8 {}
impl FromDecSpecInt<u16> for u16 {}
impl FromDecSpecInt<u32> for u32 {}
impl FromDecSpecInt<u64> for u64 {}
impl FromDecSpecInt<i8> for i8 {}
impl FromDecSpecInt<i16> for i16 {}
impl FromDecSpecInt<i32> for i32 {}
impl FromDecSpecInt<i64> for i64 {}
impl FromDecSpecInt<usize> for usize {}

#[allow(unused)]
mod test {

    use super::*;

    #[test]
    fn basic_int_signed() {
        assert_eq!(i32::from_dec_spec("1K").unwrap(), 1000);
        assert_eq!(i32::from_dec_spec("-1K").unwrap(), -1000);
        assert_eq!(i32::from_dec_spec("1.12K").unwrap(), 1120);
        assert_eq!(i32::from_dec_spec("1.1112K").unwrap(), 1111);
    }

    #[test]
    fn float() {
        assert_eq!(f32::from_dec_spec("1m").unwrap(), 0.001);
        assert_eq!(f32::from_dec_spec("1u").unwrap(), 0.000001);
        assert_eq!(f32::from_dec_spec("1μ").unwrap(), 0.000001);
        assert_eq!(f32::from_dec_spec("1234μ").unwrap(), 0.001234);
        assert_eq!(f32::from_dec_spec("2Y").unwrap(), 2e24);
        assert_eq!(f32::from_dec_spec("2y").unwrap(), 2e-24);
    }

    #[test]
    fn errors() {
        // last character is valid but number is broken
        assert!(matches!(
            i32::from_dec_spec("not_a_number_K"),
            Err(FromDecSpecError::ParseNumError)
        ));

        // number is broken but last character is valid
        assert!(matches!(
            i32::from_dec_spec("1.241o"),
            Err(FromDecSpecError::ParseError(_))
        ));

        // number does not fit into target type
        assert!(matches!(
            u16::from_dec_spec("1M"),
            Err(FromDecSpecError::ConvertNumError)
        ));
    }
}
