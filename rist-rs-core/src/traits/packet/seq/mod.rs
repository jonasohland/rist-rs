use core::fmt::{Debug, Display};
use num_traits::{Bounded, NumOps, One, Unsigned, WrappingAdd, WrappingSub, Zero};

/// Sequence number trait. This trait is implemented for 
/// u8, u16, u32 and u64
pub trait SequenceNumber:
    Unsigned
    + Bounded
    + NumOps
    + Zero
    + One
    + Ord
    + Eq
    + Display
    + Debug
    + Into<u64>
    + Copy
    + WrappingAdd
    + WrappingSub
{
}

impl SequenceNumber for u8 {}
impl SequenceNumber for u16 {}
impl SequenceNumber for u32 {}
impl SequenceNumber for u64 {}

/**
 * A packet from which a sequence number can be read
 */
pub trait OrderedPacket<S: SequenceNumber> {
    fn sequence_number(&self) -> S;
}
