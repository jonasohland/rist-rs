pub mod checksum;

use core::convert::AsMut;

macro_rules! check_bit {
    ($data:expr, $bit:expr) => {
        ($data & (1 << (7 - $bit))) != 0
    };
}

/// reads an integer from a slice of bytes in network byte order and returns the
/// integer in host byte order
macro_rules! read_int {
    ($data:expr, $t:ty, $offset:expr) => {
        <$t>::from_be_bytes(crate::bits::util::into_array(
            &$data[$offset..$offset + core::mem::size_of::<$t>()],
        ))
    };
}

pub fn into_array<A, T>(slice: &[T]) -> A
where
    A: Sized + Default + AsMut<[T]>,
    T: Clone,
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

pub(crate) use check_bit;
pub(crate) use read_int;
