#![allow(unused)]

use core::fmt::Debug;

use super::*;

#[derive(Debug)]
struct TestPacket<T: Sized> {
    seq: T,
}

impl<T> TestPacket<T> {
    fn new(seq: T) -> Self {
        Self { seq }
    }
}

impl OrderedPacket<u32> for TestPacket<u32> {
    fn sequence_number(&self) -> u32 {
        self.seq
    }
}

impl OrderedPacket<u16> for TestPacket<u16> {
    fn sequence_number(&self) -> u16 {
        self.seq
    }
}

type TestReorderBuffer<S> = ReorderRingBuffer<S, TestPacket<S>>;

/// Initialize the test environment
#[cfg(test)]
fn test_init() {
    simple_logger::init_with_level(log::Level::Trace);
}

/// Expect a packet with the given sequence number to be returned by the buffer
#[cfg(test)]
fn expect_packet<S, P>(output: ReorderQueueEvent<S, P>, seq: S)
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
    P: Debug,
{
    match output {
        ReorderQueueEvent::Packet(p) => {
            assert_eq!(p.sequence_number(), seq)
        }
        other => panic!("unexpected buffer state: {:?}", other),
    }
}

/// Expect the buffer to be reset to the provided sequence number
#[cfg(test)]
fn expect_reset<S, P>(output: ReorderQueueEvent<S, P>, seq: S)
where
    S: SequenceNumber,
    P: OrderedPacket<S>,
    P: Debug,
{
    match output {
        ReorderQueueEvent::Reset(s) => {
            assert_eq!(s, seq)
        }
        other => panic!("unexpected buffer state: {:?}", other),
    }
}

/// Send a sequence of sequence numbers into the buffer
#[cfg(test)]
fn send_seq<S>(buf: &mut impl ReorderQueueInput<S, TestPacket<S>>, seq: impl IntoIterator<Item = S>)
where
    S: SequenceNumber,
    TestPacket<S>: OrderedPacket<S>,
{
    seq.into_iter().for_each(|s| {
        assert!(buf.put(TestPacket::new(s)).is_none());
    });
}

#[test]
fn push_pop_basic() {
    test_init();
    let mut buffer = TestReorderBuffer::<u32>::new(32);
    assert!(buffer.put(TestPacket::new(0)).is_none());
    match buffer.next_event() {
        ReorderQueueEvent::Packet(p) => {
            assert_eq!(p.sequence_number(), 0);
        }
        other => {
            panic!("invalid buffer state returned: {:?}", other);
        }
    }
}

#[test]
fn push_full_buffer() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(3);
    assert!(buf.put(TestPacket::new(0)).is_none());
    assert!(buf.put(TestPacket::new(1)).is_none());
    // if the buffer is full packet should be returned [send] function
    assert_eq!(buf.put(TestPacket::new(2)).unwrap().sequence_number(), 2);
    assert_eq!(buf.len(), 2);
}

#[test]
fn detect_reset_start() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // this packet has no chance to be recovered and is considered a reset of the buffer
    buf.put(TestPacket::new(33));
    expect_reset(buf.next_event(), 33);
}

#[test]
fn detect_no_reset_start() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // this packet has no chance to be recovered and is considered a reset of the buffer
    buf.put(TestPacket::new(32));
    assert!(!matches!(buf.next_event(), ReorderQueueEvent::Reset(_)));
}

#[test]
fn detect_reset_start_wrapped() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // this packet has no chance to be recovered and is considered a reset of the buffer
    buf.put(TestPacket::new(u32::MAX - 33));
    expect_reset(buf.next_event(), u32::MAX - 33);
}

#[test]
fn detect_no_reset_start_wrapped() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // this is not a reset, but a late packet
    buf.put(TestPacket::new(u32::MAX - 31));
    assert!(!matches!(buf.next_event(), ReorderQueueEvent::Reset(_)));
}

#[test]
fn reject_late() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // check that the late packet was rejected
    assert_eq!(
        buf.put(TestPacket::new(u32::MAX - 31))
            .unwrap()
            .sequence_number(),
        u32::MAX - 31
    );
    // and the buffer was not reset
    assert!(!matches!(buf.next_event(), ReorderQueueEvent::Reset(_)));
}

#[test]
fn accept_early() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // check that and early packet is accepted
    assert!(buf.put(TestPacket::new(31)).is_none());
    assert!(buf.put(TestPacket::new(0)).is_none());
    // and the buffer was not reset
    // and the first packet is returned
    expect_packet(buf.next_event(), 0);
    // and we are now waiting for more packets
    assert!(matches!(buf.next_event(), ReorderQueueEvent::NeedMore));
    assert_eq!(buf.skip_to_next().unwrap().sequence_number(), 31);
}

#[test]
fn reorder_basic() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // push some unordered packets
    send_seq(&mut buf, [4, 1, 2, 0, 5, 3].into_iter());
    assert_eq!(buf.len(), 6);
    // get back ordered packets
    for i in 0..6u32 {
        expect_packet(buf.next_event(), i);
    }
    // no next packet has not arrived yet
    assert!(matches!(buf.next_event(), ReorderQueueEvent::NeedMore));

    // buffer should be empty now
    assert_eq!(buf.len(), 0);
}

#[test]
fn skip_and_drain() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    send_seq(&mut buf, [4, 1, 0, 5, 3, 6]);
    assert_eq!(buf.len(), 6);
    expect_packet(buf.next_event(), 0);
    expect_packet(buf.next_event(), 1);
    assert!(matches!(buf.next_event(), ReorderQueueEvent::NeedMore));
    // explicitly skip the missing packet (seq: 2) and get the next one
    assert_eq!(buf.skip_to_next().unwrap().sequence_number(), 3);
    expect_packet(buf.next_event(), 4);
    expect_packet(buf.next_event(), 5);
    expect_packet(buf.next_event(), 6);
}

#[test]
fn skip_and_drain_empty() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(32);
    // push some unordered packets with a gap and duplicates (evil!)
    send_seq(&mut buf, [3, 1, 0, 1, 0]);
    assert_eq!(buf.len(), 5);
    expect_packet(buf.next_event(), 0);
    expect_packet(buf.next_event(), 1);
    assert!(matches!(buf.next_event(), ReorderQueueEvent::NeedMore));
    // explicitly skip the missing packet (seq: 2) and get the next one
    assert_eq!(buf.skip_to_next().unwrap().sequence_number(), 3);
    assert!(matches!(buf.skip_to_next(), None));
    // now the buffer is drained
    assert!(buf.is_empty())
}

#[test]
fn missing() {
    test_init();
    let mut buf = TestReorderBuffer::<u32>::new(6);
    send_seq(&mut buf, [0, 1, 2, 4, 5]);
    expect_packet(buf.next_event(), 0);
    expect_packet(buf.next_event(), 1);
    expect_packet(buf.next_event(), 2);
    send_seq(&mut buf, [6, 7]);
    assert!(matches!(buf.next_event(), ReorderQueueEvent::Missing));
}


