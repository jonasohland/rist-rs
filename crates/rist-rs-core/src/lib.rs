#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(test)]
#[macro_use]
extern crate std;

#[allow(unused)]
pub mod net;

// #[allow(unused)]
pub mod time;

pub mod internal;

mod profiles;

mod proto;

pub mod util;

pub mod traits;

pub mod collections;
