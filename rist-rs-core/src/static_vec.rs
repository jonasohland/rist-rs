use core::{
    alloc::Allocator,
    ops::{Deref, DerefMut},
};

use alloc::alloc::Global;

pub struct StaticVec<T, A = Global>
where
    A: Allocator,
    T: Default,
{
    data: Vec<T, A>,
}

impl<T, A> StaticVec<T, A>
where
    A: Allocator,
    T: Default,
{
    pub fn new_in(len: usize, alloc: A) -> Self {
        let mut data = Vec::<T, A>::with_capacity_in(len, alloc);
        data.resize_with(len, Default::default);
        Self { data }
    }
}

impl<T, A> Deref for StaticVec<T, A>
where
    A: Allocator,
    T: Default,
{
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, A> DerefMut for StaticVec<T, A>
where
    A: Allocator,
    T: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> StaticVec<T, Global>
where
    T: Default,
{
    pub fn new(len: usize) -> Self {
        Self::new_in(len, Global)
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