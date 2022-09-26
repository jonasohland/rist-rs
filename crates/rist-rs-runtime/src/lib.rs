#![no_std]

pub mod transport;

pub trait Runtime {
    /// Return the runtime name
    fn name(&self) -> &'static str;
}
