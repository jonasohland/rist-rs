// The MIT License (MIT)
//
// Copyright (c) 2019 The Crossbeam Project Developers
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Bounded channel based on a preallocated array.
//!
//! This flavor has a fixed, positive capacity.
//!
//! The implementation is based on Dmitry Vyukov's bounded MPMC queue.
//!
//! Source:
//!   - <http://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue>
//!   - <https://docs.google.com/document/d/1yIAYmbvL3JxOKOjuCyon7JhW4cSv1wy5hC0ApeGMV9s/pub>
#![allow(unused)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{self, AtomicUsize, Ordering};

use alloc::boxed::Box;
use alloc::sync::Arc;

use crate::util::mem::cache::CachePadded;
use crate::util::sync::backoff::Backoff;

#[derive(Debug)]
pub enum TrySendError<T> {
    Full(T),
    Disconnected(T),
}

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

/// A slot in a channel.
struct Slot<T> {
    /// The current stamp.
    stamp: AtomicUsize,

    /// The message in this slot.
    msg: UnsafeCell<MaybeUninit<T>>,
}

/// The token type for the array flavor.
#[derive(Debug)]
pub(crate) struct Token {
    /// Slot to read from or write to.
    slot: *const u8,

    /// Stamp to store into the slot after reading or writing.
    stamp: usize,
}

impl Default for Token {
    #[inline]
    fn default() -> Self {
        Token {
            slot: ptr::null(),
            stamp: 0,
        }
    }
}

/// Bounded channel based on a preallocated array.
pub(crate) struct Channel<T> {
    /// The head of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit in the head is always zero.
    ///
    /// Messages are popped from the head of the channel.
    head: CachePadded<AtomicUsize>,

    /// The tail of the channel.
    ///
    /// This value is a "stamp" consisting of an index into the buffer, a mark bit, and a lap, but
    /// packed into a single `usize`. The lower bits represent the index, while the upper bits
    /// represent the lap. The mark bit indicates that the channel is disconnected.
    ///
    /// Messages are pushed into the tail of the channel.
    tail: CachePadded<AtomicUsize>,

    /// The buffer holding slots.
    buffer: Box<[Slot<T>]>,

    /// The channel capacity.
    cap: usize,

    /// A stamp with the value of `{ lap: 1, mark: 0, index: 0 }`.
    one_lap: usize,

    /// If this bit is set in the tail, that means the channel is disconnected.
    mark_bit: usize,
}

impl<T> Channel<T> {
    /// Creates a bounded channel of capacity `cap`.
    pub(crate) fn with_capacity(cap: usize) -> Self {
        assert!(cap > 0, "capacity must be positive");

        // Compute constants `mark_bit` and `one_lap`.
        let mark_bit = (cap + 1).next_power_of_two();
        let one_lap = mark_bit * 2;

        // Head is initialized to `{ lap: 0, mark: 0, index: 0 }`.
        let head = 0;
        // Tail is initialized to `{ lap: 0, mark: 0, index: 0 }`.
        let tail = 0;

        // Allocate a buffer of `cap` slots initialized
        // with stamps.
        let buffer: Box<[Slot<T>]> = (0..cap)
            .map(|i| {
                // Set the stamp to `{ lap: 0, mark: 0, index: i }`.
                Slot {
                    stamp: AtomicUsize::new(i),
                    msg: UnsafeCell::new(MaybeUninit::uninit()),
                }
            })
            .collect();

        Channel {
            buffer,
            cap,
            one_lap,
            mark_bit,
            head: CachePadded::new(AtomicUsize::new(head)),
            tail: CachePadded::new(AtomicUsize::new(tail)),
        }
    }

    fn start_send(&self, token: &mut Token) -> bool {
        let backoff = Backoff::new();
        let mut tail = self.tail.load(Ordering::Relaxed);

        loop {
            // Check if the channel is disconnected.
            if tail & self.mark_bit != 0 {
                token.slot = ptr::null();
                token.stamp = 0;
                return true;
            }

            // Deconstruct the tail.
            let index = tail & (self.mark_bit - 1);
            let lap = tail & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            debug_assert!(index < self.buffer.len());
            let slot = unsafe { self.buffer.get_unchecked(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the tail and the stamp match, we may attempt to push.
            if tail == stamp {
                let new_tail = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, mark: 0, index: index + 1 }`.
                    tail + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), mark: 0, index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the tail.
                match self.tail.compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Prepare the token for the follow-up call to `write`.
                        token.slot = slot as *const Slot<T> as *const u8;
                        token.stamp = tail + 1;
                        return true;
                    }
                    Err(t) => {
                        tail = t;
                        backoff.spin();
                    }
                }
            } else if stamp.wrapping_add(self.one_lap) == tail + 1 {
                atomic::fence(Ordering::SeqCst);
                let head = self.head.load(Ordering::Relaxed);

                // If the head lags one lap behind the tail as well...
                if head.wrapping_add(self.one_lap) == tail {
                    // ...then the channel is full.
                    return false;
                }

                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
                backoff.snooze();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }

    /// Writes a message into the channel.
    pub(crate) unsafe fn write(&self, token: &mut Token, msg: T) -> Result<(), T> {
        // If there is no slot, the channel is disconnected.
        if token.slot.is_null() {
            return Err(msg);
        }

        let slot: &Slot<T> = &*token.slot.cast::<Slot<T>>();

        // Write the message into the slot and update the stamp.
        slot.msg.get().write(MaybeUninit::new(msg));
        slot.stamp.store(token.stamp, Ordering::Release);

        Ok(())
    }

    /// Attempts to reserve a slot for receiving a message.
    fn start_recv(&self, token: &mut Token) -> bool {
        let backoff = Backoff::new();
        let mut head = self.head.load(Ordering::Relaxed);

        loop {
            // Deconstruct the head.
            let index = head & (self.mark_bit - 1);
            let lap = head & !(self.one_lap - 1);

            // Inspect the corresponding slot.
            debug_assert!(index < self.buffer.len());
            let slot = unsafe { self.buffer.get_unchecked(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            // If the the stamp is ahead of the head by 1, we may attempt to pop.
            if head + 1 == stamp {
                let new = if index + 1 < self.cap {
                    // Same lap, incremented index.
                    // Set to `{ lap: lap, mark: 0, index: index + 1 }`.
                    head + 1
                } else {
                    // One lap forward, index wraps around to zero.
                    // Set to `{ lap: lap.wrapping_add(1), mark: 0, index: 0 }`.
                    lap.wrapping_add(self.one_lap)
                };

                // Try moving the head.
                match self.head.compare_exchange_weak(
                    head,
                    new,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Prepare the token for the follow-up call to `read`.
                        token.slot = slot as *const Slot<T> as *const u8;
                        token.stamp = head.wrapping_add(self.one_lap);
                        return true;
                    }
                    Err(h) => {
                        head = h;
                        backoff.spin();
                    }
                }
            } else if stamp == head {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.load(Ordering::Relaxed);

                // If the tail equals the head, that means the channel is empty.
                if (tail & !self.mark_bit) == head {
                    // If the channel is disconnected...
                    if tail & self.mark_bit != 0 {
                        // ...then receive an error.
                        token.slot = ptr::null();
                        token.stamp = 0;
                        return true;
                    } else {
                        // Otherwise, the receive operation is not ready.
                        return false;
                    }
                }

                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                // Snooze because we need to wait for the stamp to get updated.
                backoff.snooze();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }

    /// Reads a message from the channel.
    pub(crate) unsafe fn read(&self, token: &mut Token) -> Result<T, ()> {
        if token.slot.is_null() {
            // The channel is disconnected.
            return Err(());
        }

        let slot: &Slot<T> = &*token.slot.cast::<Slot<T>>();

        // Read the message from the slot and update the stamp.
        let msg = slot.msg.get().read().assume_init();
        slot.stamp.store(token.stamp, Ordering::Release);

        Ok(msg)
    }

    /// Attempts to send a message into the channel.
    pub(crate) fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        let token = &mut Token::default();
        if self.start_send(token) {
            unsafe { self.write(token, msg).map_err(TrySendError::Disconnected) }
        } else {
            Err(TrySendError::Full(msg))
        }
    }

    /// Attempts to receive a message without blocking.
    pub(crate) fn try_recv(&self) -> Result<T, TryRecvError> {
        let token = &mut Token::default();

        if self.start_recv(token) {
            unsafe { self.read(token).map_err(|_| TryRecvError::Disconnected) }
        } else {
            Err(TryRecvError::Empty)
        }
    }

    /// Disconnects the channel and wakes up all blocked senders and receivers.
    ///
    /// Returns `true` if this call disconnected the channel.
    pub(crate) fn disconnect(&self) -> bool {
        let tail = self.tail.fetch_or(self.mark_bit, Ordering::SeqCst);
        tail & self.mark_bit == 0
    }

    /// Returns `true` if the channel is disconnected.
    pub(crate) fn is_disconnected(&self) -> bool {
        self.tail.load(Ordering::SeqCst) & self.mark_bit != 0
    }

    /// Returns `true` if the channel is empty.
    pub(crate) fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        // Is the tail equal to the head?
        //
        // Note: If the head changes just before we load the tail, that means there was a moment
        // when the channel was not empty, so it is safe to just return `false`.
        (tail & !self.mark_bit) == head
    }

    /// Returns `true` if the channel is full.
    pub(crate) fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);

        // Is the head lagging one lap behind tail?
        //
        // Note: If the tail changes just before we load the head, that means there was a moment
        // when the channel was not full, so it is safe to just return `false`.
        head.wrapping_add(self.one_lap) == tail & !self.mark_bit
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // Get the index of the head.
        let head = *self.head.get_mut();
        let tail = *self.tail.get_mut();

        let hix = head & (self.mark_bit - 1);
        let tix = tail & (self.mark_bit - 1);

        let len = if hix < tix {
            tix - hix
        } else if hix > tix {
            self.cap - hix + tix
        } else if (tail & !self.mark_bit) == head {
            0
        } else {
            self.cap
        };

        // Loop over all slots that hold a message and drop them.
        for i in 0..len {
            // Compute the index of the next slot holding a message.
            let index = if hix + i < self.cap {
                hix + i
            } else {
                hix + i - self.cap
            };

            unsafe {
                debug_assert!(index < self.buffer.len());
                let slot = self.buffer.get_unchecked_mut(index);
                let msg = &mut *slot.msg.get();
                msg.as_mut_ptr().drop_in_place();
            }
        }
    }
}

/// Receiver handle to a channel.
pub(crate) struct ReceiveEnd<T>(Arc<Channel<T>>);

impl<T> Drop for ReceiveEnd<T> {
    fn drop(&mut self) {
        self.0.disconnect();
    }
}

/// Receiver handle to a channel.
pub(crate) struct SendEnd<T>(Arc<Channel<T>>);

impl<T> Drop for SendEnd<T> {
    fn drop(&mut self) {
        self.0.disconnect();
    }
}

#[derive(Clone)]
pub struct Receiver<T>(Arc<ReceiveEnd<T>>);

#[derive(Clone)]
pub struct Sender<T>(Arc<SendEnd<T>>);

impl<T> Receiver<T> {
    #[rustfmt::skip]
    pub fn try_receive(&self) -> Result<T, TryRecvError> {
        self.0.0.try_recv()
    }

    pub fn is_disconnected(&self) -> bool {
        self.0 .0.is_disconnected()
    }
}

impl<T> Sender<T> {
    #[rustfmt::skip]
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.0.0.try_send(msg)
    }

    pub fn is_disconnected(&self) -> bool {
        self.0 .0.is_disconnected()
    }
}

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let ch = Arc::new(Channel::with_capacity(cap));
    (
        Sender(Arc::new(SendEnd(ch.clone()))),
        Receiver(Arc::new(ReceiveEnd(ch))),
    )
}

unsafe impl<T> Send for Sender<T> {}
unsafe impl<T> Sync for Sender<T> {}
unsafe impl<T> Send for Receiver<T> {}
unsafe impl<T> Sync for Receiver<T> {}

#[test]
fn test() {
    let (tx, rx) = channel(10);
    let tx_th = std::thread::spawn(move || {
        for i in 0..12345678 {
            loop {
                match tx.try_send(i) {
                    Ok(_) => break,
                    Err(e) => {
                        if let TrySendError::Disconnected(_) = e {
                            panic!()
                        }
                    }
                }
            }
        }
    });

    let rx_th = std::thread::spawn(move || {
        for i in 0..12345678 {
            loop {
                match rx.try_receive() {
                    Ok(v) => {
                        assert_eq!(v, i);
                        break;
                    }
                    Err(e) => {
                        if matches!(e, TryRecvError::Disconnected) {
                            panic!()
                        }
                    }
                }
            }
        }
    });

    tx_th.join().unwrap();
    rx_th.join().unwrap();
}
