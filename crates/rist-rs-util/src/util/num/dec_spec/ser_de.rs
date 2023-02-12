#![allow(unused)]
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};

use super::*;

#[derive(Clone, Copy)]
pub struct DecSpecInt<N: FromDecSpecInt<N>>(N);

#[derive(Clone, Copy)]
pub struct DecSpecFloat<N: FromDecSpecFloat<N>>(N);

impl<'de, N: FromDecSpecInt<N>> Deserialize<'de> for DecSpecInt<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecSpecIntVisitor<T>(PhantomData<T>);

        impl<'de, T: FromDecSpecInt<T>> Visitor<'de> for DecSpecIntVisitor<T> {
            type Value = T;

            fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
                formatter.write_str("string or number")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::from_dec_spec(v).map_err(|e| serde::de::Error::custom(e))
            }

            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::try_from(v)
                    .map_err(|_| serde::de::Error::custom("value out of range for target type"))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_i128(v as i128)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_i128(v as i128)
            }
            
        }
        deserializer
            .deserialize_any(DecSpecIntVisitor(PhantomData))
            .map(DecSpecInt)
    }
}

impl<N: FromDecSpecInt<N>> Serialize for DecSpecInt<N>
where
    N: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, N: FromDecSpecFloat<N>> Deserialize<'de> for DecSpecFloat<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecSpecFloatVisitor<T>(PhantomData<T>);

        impl<'de, T: FromDecSpecFloat<T>> Visitor<'de> for DecSpecFloatVisitor<T> {
            type Value = T;

            fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
                formatter.write_str("string or number")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::from_dec_spec(v).map_err(|e| serde::de::Error::custom(e))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(T::from_f64(v))
            }
        }
        deserializer
            .deserialize_any(DecSpecFloatVisitor(PhantomData))
            .map(DecSpecFloat)
    }
}

impl<N: FromDecSpecFloat<N>> Serialize for DecSpecFloat<N>
where
    N: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

macro_rules! impl_dec_spec_wrapper {
    ($trait:tt, $wrapper:tt) => {
        impl<N> Debug for $wrapper<N>
        where
            N: $trait<N>,
            N: Debug,
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<N> Display for $wrapper<N>
        where
            N: $trait<N>,
            N: Display,
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<N: $trait<N>> $wrapper<N> {
            pub fn into_inner(self) -> N {
                self.0
            }
        }

        impl<N: $trait<N>> $wrapper<N>
        where
            N: Copy,
        {
            pub fn get(&self) -> N {
                self.0
            }
        }
    };
}

impl_dec_spec_wrapper!(FromDecSpecInt, DecSpecInt);
impl_dec_spec_wrapper!(FromDecSpecFloat, DecSpecFloat);
