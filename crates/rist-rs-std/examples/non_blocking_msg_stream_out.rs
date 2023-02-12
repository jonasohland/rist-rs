use std::collections::LinkedList;
use std::env;
use std::net::SocketAddr;
use std::process::exit;

use rist_rs_types::traits::io::{ReceiveNonBlocking, SendNonBlocking};
use rist_rs_util::collections::static_vec::StaticVec;

use rist_rs_std::transport::socket::NonBlockingUdpSocket;
use rist_rs_util::stream::message::NonBlockingMessageStreamConnector;

use tracing::{debug, error};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    let mut args = env::args();
    args.next().expect("missing listen address argument");
    let mut connector = NonBlockingMessageStreamConnector::new(
        NonBlockingUdpSocket::bind(
            args.next()
                .expect("missing listen address argument")
                .parse::<SocketAddr>()
                .expect("failed to parse listen address argument"),
        )
        .unwrap(),
        1524,
    );
    let mut streams = args
        .map(|addr_s| {
            connector.connect(
                addr_s
                    .parse::<SocketAddr>()
                    .expect("failed to parse remote socket address"),
            )
        })
        .collect::<LinkedList<_>>();
    let mut rx_buf = StaticVec::<u8>::new(1500);
    for stream in &mut streams {
        stream.try_send(&[1, 2, 3, 4]);
    }
    loop {
        if let Some(Err(e)) = connector.run() {
            error!(error = ?e, "io error");
            exit(1)
        }
        streams = streams
            .into_iter()
            .filter_map(|mut stream| match stream.try_recv(&mut rx_buf) {
                Some(res) => match res {
                    Ok(len) => {
                        debug!(peer = ?stream.peer_address(), len, "response received");
                        None
                    }
                    Err(_) => {
                        error!("stream closed");
                        None
                    }
                },
                None => Some(stream),
            })
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(10));
        if streams.is_empty() {
            break;
        }
    }
}
