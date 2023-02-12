// enable std feature
#![cfg_attr(not(feature = "std"), no_std)]

// enable features dependent on alloc
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
pub mod gre;
pub mod ip;
pub mod rist;
pub mod rtcp;
pub mod rtp;
pub mod udp;
pub mod util;
