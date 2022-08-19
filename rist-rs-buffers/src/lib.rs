#![no_std]

#![feature(allocator_api)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(test)]
#[macro_use]
extern crate std;

/// Buffers that can be used for packet reordering
pub mod reorder;
pub mod rist;