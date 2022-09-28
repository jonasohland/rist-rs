use std::collections::LinkedList;
use std::env;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use rist_rs_core::collections::static_vec::StaticVec;
use rist_rs_core::traits::io::{ReceiveNonBlocking, SendNonBlocking};

use rist_rs_buffers::stream::message::{
    NonBlockingMessageStream, NonBlockingMessageStreamAcceptor,
};
use rist_rs_runtime_std::transport::socket::NonBlockingUdpSocket;

use tracing::{debug, error};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    let mut args = env::args();
    args.next().expect("missing listen address argument");
    let mut acceptor = NonBlockingMessageStreamAcceptor::new(
        NonBlockingUdpSocket::bind(
            args.next()
                .expect("missing listen address argument")
                .parse::<SocketAddr>()
                .expect("failed to parse listen address argument"),
        )
        .unwrap(),
        1524,
    );
    let mut streams = LinkedList::<(NonBlockingMessageStream<SocketAddr>, SystemTime)>::new();
    let mut rx_buf = StaticVec::<u8>::new(1500);
    loop {
        if let Some(res) = acceptor.accept() {
            match res {
                Ok(stream) => streams.push_back((stream, SystemTime::now())),
                Err(e) => {
                    error!("accept failed: {}", e);
                    return;
                }
            }
        }
        streams = streams
            .into_iter()
            .filter_map(|(mut stream, last_packet)| {
                let now = SystemTime::now();
                match stream.try_recv(&mut rx_buf) {
                    Some(res) => match res {
                        Ok(len) => {
                            match stream.try_send(rx_buf.split_at(len).0) {
                                Some(Err(_)) | None => {
                                    error!("write failed");
                                }
                                _ => {}
                            }
                            Some((stream, now))
                        }
                        Err(_) => {
                            error!("stream closed");
                            None
                        }
                    },
                    None => {
                        if now.duration_since(last_packet).unwrap() > Duration::from_secs(30) {
                            debug!("stream timed out");
                            None
                        } else {
                            Some((stream, last_packet))
                        }
                    }
                }
            })
            .collect();

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
