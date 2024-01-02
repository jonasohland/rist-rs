use core::fmt::Debug;

use super::{Event, Runtime};
use crate::traits::protocol::Ctl;

#[derive(Debug)]
pub struct Events<R: Runtime, C: Ctl>(Vec<Event<R, C>>);

impl<R: Runtime, C: Ctl> Events<R, C> {
    /// Creates a new event buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Push a new event to the buffer
    pub fn push(&mut self, ev: Event<R, C>) {
        self.0.push(ev)
    }

    /// Pop an event from the back of the buffer
    pub fn pop(&mut self) -> Option<Event<R, C>> {
        self.0.pop()
    }

    /// Return the available capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns true if no events can be popped off of the back of the buffer
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of events available
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Reserve the given capacity for events to be pushed before internal re-allocation takes place
    pub fn reserve(&mut self, len: usize) {
        self.0.reserve_exact(len)
    }
}
