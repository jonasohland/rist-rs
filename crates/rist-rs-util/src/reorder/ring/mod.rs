// #![allow(unused)]

use crate::collections::static_vec::StaticVec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use rist_rs_types::traits::{
    packet::seq::{OrderedPacket, SequenceNumber},
    queue::reorder::{ReorderQueueEvent, ReorderQueueInput, ReorderQueueOutput},
};

trait Flag: Sized + Copy {
    fn swap(&mut self, val: Self) -> Self {
        let before = *self;
        *self = val;
        before
    }
}

impl Flag for bool {}

#[derive(Debug, Default, Clone)]
pub struct ReorderRingBufferMetrics {
    pub lost: u64,
    pub delivered: u64,
    pub dropped: u64,
    pub rejected: u64,
    pub reordered: u64,
}
pub struct ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    /// Internal metrics
    metrics: ReorderRingBufferMetrics,

    /// holds all the packet data
    data: StaticVec<Option<P>>,

    /// write-head position
    write_pos: usize,

    /// read-head position
    read_pos: usize,

    /// current sequence number the reader is currently waiting for
    read_seq: S,

    /// the last-written sequence number
    write_seq: S,

    /// flag that indicates that the packet sequence has reset and the buffer was cleared
    reset_flag: bool,

    // ignores the generic argument
    _p: PhantomData<S>,
}

impl<S, P> ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    #[allow(unused)]
    pub fn new(len: usize) -> Self {
        Self {
            metrics: ReorderRingBufferMetrics::default(),
            data: StaticVec::new(len),
            write_pos: 0,
            read_pos: 0,
            read_seq: S::zero(),
            write_seq: S::zero(),
            reset_flag: false,
            _p: Default::default(),
        }
    }
}

impl<S, P> ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    /// Set a iterator position to the next field in the buffer
    fn advance(pos: &mut usize, len: usize) -> usize {
        let pos_last = *pos;
        *pos += 1;
        if *pos >= len {
            *pos = 0;
        }
        pos_last
    }

    /// Increment sequence number to be read
    fn next_read_seq(&mut self) {
        self.read_seq = self.read_seq.wrapping_add(&S::one());
    }

    /// Read and reset the reset-flag
    fn read_reset_flag(&mut self) -> bool {
        self.reset_flag.swap(false)
    }

    /// Advance the write-head by one
    fn advance_write_head(&mut self) -> usize {
        Self::advance(&mut self.write_pos, self.data.len())
    }

    /// Advance the read-head by one
    fn advance_read_head(&mut self) -> usize {
        Self::advance(&mut self.read_pos, self.data.len())
    }

    /// Check if the given position is valid to read
    fn valid_to_read(&self, pos: usize) -> bool {
        self.write_pos != pos
    }

    /// Check if the buffer can be written to
    fn can_write(&self) -> bool {
        self.write_pos
            != (self.read_pos as i64 - 1 + self.data.len() as i64) as usize % self.data.len()
    }

    /// Check if the sequence was reset
    fn is_seq_reset(&self, last: S, current: S) -> bool {
        let diff: u64 = if current < last {
            last - current
        } else {
            current - last
        }
        .into();
        diff > self.data.len() as u64 && diff <= (S::max_value().into() - self.data.len() as u64)
    }

    /// Try to push a packet to the buffer. Returns the packet if it could not be pushed
    fn push(&mut self, packet: P) -> Option<P> {
        if self.can_write() {
            let pos = self.advance_write_head();
            if let Some(s) = self.data[pos].replace(packet) {
                tracing::trace!(
                    "dropped a packet (overwrite) with sequence number: {:?}",
                    s.sequence_number()
                );
            }
            None
        } else {
            Some(packet)
        }
    }

    /// Check if a packet is expired and cannot be read any more
    fn is_expired(read_seq: S, packet: &P) -> bool {
        read_seq != packet.sequence_number() && {
            let pivot = S::max_value() / (S::one() + S::one());
            let ps = packet.sequence_number();
            if ps > read_seq {
                ps - read_seq > pivot
            } else {
                read_seq - ps < pivot
            }
        }
    }

    /// Try popping the next packet from the buffer. Returns None if the packet was not found
    /// This will also remove expired packets from the buffer and advance the read-head
    /// if trailing packets can be removed
    fn try_pop(&mut self) -> Option<P> {
        let mut cursor = self.read_pos;
        loop {
            // check if we are allowed to read this
            if !self.valid_to_read(cursor) {
                break None;
            }
            match &mut self.data[cursor] {
                Some(p) => {
                    if p.sequence_number() == self.read_seq {
                        if cursor != self.read_pos {
                            self.metrics.reordered += 1;
                        }
                        break self.data[cursor].take();
                    }
                    if Self::is_expired(self.read_seq, p) {
                        tracing::trace!(
                            "dropped a packet with sequence number: {:?}",
                            p.sequence_number()
                        );
                        // advance the write head if we have not skipped any packets yet
                        if cursor == self.read_pos {
                            self.advance_read_head();
                        }
                        self.metrics.dropped += 1;
                        drop(self.data[cursor].take());
                    }
                }
                None => {
                    // advance the write head if we have not skipped any packets yet
                    if cursor == self.read_pos {
                        self.advance_read_head();
                    }
                }
            }
            // go to next packet
            Self::advance(&mut cursor, self.data.len());
        }
    }
}

impl<S, P> ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    /// Reset the buffer and set the next sequence number to be read
    #[allow(unused)]
    pub fn reset(&mut self, s: S) {
        for cell in self.data.iter_mut() {
            drop(cell.take())
        }
        self.read_pos = 0;
        self.write_pos = 0;
        self.read_seq = s.wrapping_sub(&S::one());
        self.write_seq = s;
        self.reset_flag = true
    }

    /// Number of elements in the buffer
    #[allow(unused)]
    pub fn len(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            (self.data.len() - self.read_pos) + self.write_pos
        }
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Skip missing packets and return the next packet in the sequence
    #[allow(unused)]
    pub fn skip_to_next(&mut self) -> Option<P> {
        loop {
            match self.try_pop() {
                None => match self.len() {
                    0 => break None,
                    _ => self.next_read_seq(),
                },
                Some(p) => {
                    self.next_read_seq();
                    break Some(p);
                }
            }
        }
    }

    /// Sequence number currently read
    pub fn current_read_seq(&self) -> S {
        self.read_seq
    }

    pub fn debug_cell_diffs(&self) -> Vec<Option<i64>> {
        let current = self.read_seq.into() as i64;
        self.data
            .iter()
            .map(|cell| {
                cell.as_ref()
                    .map(|p| current - p.sequence_number().into() as i64)
            })
            .collect()
    }

    /// Accumulated metrics
    #[allow(unused)]
    pub fn metrics(&self) -> ReorderRingBufferMetrics {
        self.metrics.clone()
    }
}

impl<S, P> ReorderQueueInput<S, P> for ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
    u64: From<S>,
{
    #[allow(unused)]
    fn put(&mut self, packet: P) -> Option<P> {
        let s = packet.sequence_number();
        if self.is_seq_reset(self.write_seq, s) {
            tracing::debug!("reset buffer from previous seq {} -> {}", self.write_seq, s);
            self.reset(packet.sequence_number())
        }
        if !Self::is_expired(self.read_seq, &packet) {
            self.write_seq = s;
            self.push(packet)
        } else {
            tracing::trace!(
                "reject expired packet with sequence number: {}",
                packet.sequence_number()
            );
            self.metrics.rejected += 1;
            Some(packet)
        }
    }
}

impl<S, P> ReorderQueueOutput<S, P> for ReorderRingBuffer<S, P>
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
{
    #[allow(unused)]
    fn next_event(&mut self) -> ReorderQueueEvent<S, P> {
        if self.read_reset_flag() {
            self.next_read_seq();
            ReorderQueueEvent::Reset(self.read_seq)
        } else {
            match self.try_pop() {
                Some(p) => {
                    self.next_read_seq();
                    self.metrics.delivered += 1;
                    ReorderQueueEvent::Packet(p)
                }
                None => {
                    if self.len() + 2 >= self.data.len() {
                        self.next_read_seq();
                        self.metrics.lost += 1;
                        ReorderQueueEvent::Missing
                    } else {
                        ReorderQueueEvent::NeedMore
                    }
                }
            }
        }
    }
}

mod test;
