use core::fmt::Debug;

use crate::traits::protocol::Ctl;

use super::{Error, Runtime};

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct Readiness: u32 {
        const Writable = 0x01;
        const Readable = 0x02;
    }
}

pub enum Event<R: Runtime, C: Ctl> {
    Ready(R::Socket, Readiness),
    Error(R::Socket, Error),
    Ctl(C),
    Empty,
}

impl<R: Runtime, C: Ctl> Clone for Event<R, C> {
    fn clone(&self) -> Self {
        match self {
            Self::Ready(arg0, arg1) => Self::Ready(*arg0, *arg1),
            Self::Error(arg0, arg1) => Self::Error(*arg0, arg1.clone()),
            Self::Ctl(arg0) => Self::Ctl(arg0.clone()),
            Self::Empty => Self::Empty,
        }
    }
}

impl<R: Runtime, C: Ctl> Debug for Event<R, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(arg0, arg1) => f.debug_tuple("Readable").field(arg0).field(arg1).finish(),
            Self::Error(arg0, arg1) => f.debug_tuple("Error").field(arg0).field(arg1).finish(),
            Self::Ctl(arg0) => f.debug_tuple("Ctl").field(arg0).finish(),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl<R: Runtime, C: Ctl> Default for Event<R, C> {
    fn default() -> Self {
        Self::Empty
    }
}
