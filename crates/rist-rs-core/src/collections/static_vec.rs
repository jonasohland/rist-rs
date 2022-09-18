use core::ops::{Deref, DerefMut};

use alloc::vec::Vec;

pub struct StaticVec<T>
where
    T: Default,
{
    data: Vec<T>,
}

impl<T> StaticVec<T>
where
    T: Default,
{
    pub fn new(len: usize) -> Self {
        let mut data = Vec::<T>::with_capacity(len);
        data.resize_with(len, Default::default);
        Self { data }
    }
}

impl<T> Deref for StaticVec<T>
where
    T: Default,
{
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for StaticVec<T>
where
    T: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[test]
fn test() {
    let static_vec = StaticVec::<u32>::new(32);
    assert_eq!(static_vec.len(), 32);
    for n in static_vec.iter() {
        assert_eq!(*n, 0);
    }
}
