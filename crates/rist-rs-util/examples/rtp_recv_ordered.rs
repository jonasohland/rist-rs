use rist_rs_bits::rtp::{error::Error as RTPError, RTPView};
use rist_rs_types::traits::{
    packet::seq::OrderedPacket,
    queue::reorder::{ReorderQueueEvent, ReorderQueueInput, ReorderQueueOutput},
};
use rist_rs_util::{collections::static_vec::StaticVec, reorder::ring::ReorderRingBuffer};
use std::{
    env,
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    process::exit,
    str::FromStr,
    time::{Duration, SystemTime},
};

struct RTPPacket {
    payload: Vec<u8>,
    sequence_number: u16,
}

impl RTPPacket {
    pub fn try_new<'a, T, U>(buf: &'a T) -> Result<Self, RTPError>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let view = RTPView::try_new(buf)?;
        Ok(RTPPacket {
            payload: view.payload()?.into(),
            sequence_number: view.sequence_number(),
        })
    }
}

impl OrderedPacket<u16> for RTPPacket {
    fn sequence_number(&self) -> u16 {
        self.sequence_number
    }
}

#[derive(Default)]
struct StreamMetics {
    bytes_in: u64,
    bytes_out: u64,
}

fn report_metrics(
    buf: &ReorderRingBuffer<u16, RTPPacket>,
    stream: &mut StreamMetics,
    last_tp: SystemTime,
    now: SystemTime,
) {
    let metrics = buf.metrics();
    let que = now.duration_since(last_tp).unwrap();
    tracing::info!(
        "reorder metrics: [ok: {}] [drop: {}] [lost: {}] [ord: {}] [rej: {}] [in: {:.1}kbit/s] [out: {:.1}kbit/s]",
        metrics.delivered,
        metrics.dropped,
        metrics.lost,
        metrics.reordered,
        metrics.rejected,
        (stream.bytes_in * 8) as f64 / que.as_secs_f64() / 1000f64,
        (stream.bytes_out * 8) as f64 / que.as_secs_f64() / 1000f64
    );
    stream.bytes_in = 0;
    stream.bytes_out = 0;
}

fn main() -> ! {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    let mut args = env::args();
    drop(args.next());

    // first argument: listen address (required)
    let sock_addr: SocketAddrV4 = args
        .next()
        .expect("Missing <address>")
        .as_str()
        .parse()
        .expect("Failed to parse ip address");

    // second argument: multicast group (required but can be an empty string)
    let mcast_group = args
        .next()
        .and_then(|s| (!s.is_empty()).then_some(s))
        .map(|s| {
            <Ipv4Addr as FromStr>::from_str(&s).expect("Failed to parse <MulticastGroup> argument")
        });
    if let Some(addr) = mcast_group {
        if !addr.is_multicast() {
            panic!("[{addr}] is not a multicast group")
        }
    }

    // third argument (optional) forward address
    let tx_sock = args.next().map(|s| {
        let tx_sock = UdpSocket::bind(SocketAddrV4::new(*sock_addr.ip(), 0))
            .expect("failed to bind tx socket");
        tx_sock.connect(s).expect("failed to connect udp socket");
        tx_sock
    });

    // bind rx socket
    let rx_sock = UdpSocket::bind(sock_addr).expect("Failed to bind udp socket");

    // join multicast group
    if let Some(addr) = mcast_group {
        tracing::info!("join multicast group: {}:{}", addr, sock_addr.port());
        rx_sock
            .join_multicast_v4(&addr, sock_addr.ip())
            .expect("Failed to join multicast group");
    }

    // buffer in which packets are received
    let mut buf = StaticVec::<u8>::new(1596);

    // reordering buffer
    let mut reorder_buf = ReorderRingBuffer::<u16, RTPPacket>::new(30);

    // metrics collection and timing
    let mut stream_metrics = StreamMetics::default();
    let metrics_after = Duration::from_millis(500);
    let mut metrics_last = SystemTime::now();

    loop {
        // periodically print metrics
        let now = SystemTime::now();
        if now > metrics_last + metrics_after {
            report_metrics(&reorder_buf, &mut stream_metrics, metrics_last, now);
            metrics_last = now;
        }

        // receive next packet
        match rx_sock.recv(&mut buf) {
            Ok(data) => match RTPPacket::try_new(buf.split_at(data).0) {
                Err(e) => {
                    tracing::error!("invalid rtp packet: {:?}", e);
                }
                // send to reorder buffer
                Ok(packet) => {
                    stream_metrics.bytes_in += packet.payload.len() as u64;
                    reorder_buf.put(packet);
                }
            },
            Err(e) => {
                tracing::error!("recv error: {}", e);
                exit(1);
            }
        }
        // receive events from reorder buffer
        loop {
            match reorder_buf.next_event() {
                // packet received from buffer
                ReorderQueueEvent::Packet(p) => {
                    match &tx_sock {
                        // forward
                        Some(sock) => match sock.send(&p.payload) {
                            Ok(s) => {
                                stream_metrics.bytes_out += s as u64;
                            }
                            Err(e) => {
                                tracing::error!("send error: {}", e);
                                exit(1)
                            }
                        },
                        // just update metrics
                        None => {
                            stream_metrics.bytes_out += p.payload.len() as u64;
                        }
                    }
                    // check for more events
                    continue;
                }
                // no more packets available, try receiving more
                ReorderQueueEvent::NeedMore => break,
                // the next packet is considered missing, check next event from the buffer
                ReorderQueueEvent::Missing => continue,
                // sequence number was reset
                ReorderQueueEvent::Reset(s) => {
                    tracing::info!("buffer indicated a reset to sequence number: {}", s);
                    // check the next event from the buffer
                    continue;
                }
            }
        }
    }
}
