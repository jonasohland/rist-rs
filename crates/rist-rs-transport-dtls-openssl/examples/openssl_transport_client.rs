use rist_rs_std::transport::socket::NonBlockingUdpSocket;
use rist_rs_transport_dtls_openssl::transport::stream::non_blocking::{
    DtlsStream, DtlsStreamConnector, DtlsStreamError,
};
use rist_rs_transport_dtls_openssl::transport::stream::{self, SimpleContextProvider};
use rist_rs_types::traits::io::ReceiveNonBlocking;
use rist_rs_util::collections::static_vec::StaticVec;

use std::fs;
use std::net::SocketAddr;
use std::str::from_utf8;

use clap::Parser;

use openssl::pkey;
use openssl::x509;

#[derive(Parser)]
struct Config {
    #[clap(short, long, help = "Bind local socket to this address")]
    bind_address: Option<SocketAddr>,

    #[clap(short, long, help = "Listen on this address")]
    address: SocketAddr,

    #[clap(long, short, help = "Load server certificate from this file")]
    cert: String,

    #[clap(long, short, help = "Load server key from this file")]
    key: String,

    #[clap(long, short = 'C', help = "Ca certificate")]
    ca: Option<String>,

    #[clap(
        long,
        short = 'H',
        help = "Set client hostname that should be verified"
    )]
    server_hostname: Option<String>,
}

fn load_stream_config(config: &Config) -> stream::Config<SimpleContextProvider> {
    let mut builder = SimpleContextProvider::build_client()
        .unwrap()
        .with_certificate(
            &x509::X509::from_pem(
                fs::read_to_string(&config.cert)
                    .expect("failed to read private key file")
                    .as_bytes(),
            )
            .expect("failed to parse certificate file"),
        )
        .expect("failed to load certificate")
        .with_key(
            &pkey::PKey::private_key_from_pem(
                fs::read_to_string(&config.key)
                    .expect("failed to read private key file")
                    .as_bytes(),
            )
            .expect("failed to parse private key file"),
        )
        .expect("failed to set private key");
    if let Some(name) = &config.server_hostname {
        builder = builder
            .with_expected_peer_name(name)
            .expect("failed to set expected client hostname")
    }
    if let Some(path) = &config.ca {
        builder = builder
            .with_ca_cert(
                &x509::X509::from_pem(
                    fs::read_to_string(path)
                        .expect("failed to read ca certificate file")
                        .as_bytes(),
                )
                .expect("failed to parse ca certificate file"),
            )
            .expect("failed to add ca certificate")
    }
    stream::Config::new(builder.build())
}

enum StreamPhase {
    Active(DtlsStream<SocketAddr>),
    Shutdown(DtlsStream<SocketAddr>),
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    let config = Config::parse();
    let mut connector = DtlsStreamConnector::new(
        NonBlockingUdpSocket::bind(
            config
                .bind_address
                .unwrap_or_else(|| "0.0.0.0:0".parse::<SocketAddr>().unwrap()),
        )
        .unwrap(),
        2670,
        load_stream_config(&config),
    );
    if let Some(Err(error)) = connector.add(config.address) {
        tracing::error!(?error, "failed to initialize dtls connector");
    }
    let mut rx_buf = StaticVec::new(65536);
    let mut connection = None;
    loop {
        match connector.try_connect() {
            Some(Ok(connections)) => {
                connection = connections.into_iter().next().map(StreamPhase::Active);
            }
            Some(Err(error)) => {
                tracing::error!(?error, "connection failed");
                break;
            }
            None => {
                if let Some(phase) = connection.take() {
                    match phase {
                        StreamPhase::Active(mut stream) => match stream.try_recv(&mut rx_buf) {
                            Some(Ok(len)) => {
                                tracing::info!(
                                    "got: {}",
                                    from_utf8(rx_buf.split_at(len).0).unwrap()
                                );
                                connection = Some(StreamPhase::Active(stream))
                            }
                            Some(Err(DtlsStreamError::Ssl(error))) => {
                                if !stream.shutdown_state().is_active() {
                                    connection = Some(StreamPhase::Shutdown(stream))
                                } else {
                                    tracing::error!(%error, "ssl error");
                                    return;
                                }
                            }
                            Some(Err(error)) => {
                                tracing::error!(?error, "stream error");
                            }
                            None => connection = Some(StreamPhase::Active(stream)),
                        },
                        StreamPhase::Shutdown(mut stream) => match stream.shutdown() {
                            Some(Ok(_)) => {
                                return;
                            }
                            Some(Err(error)) => {
                                tracing::error!(?error, "shutdown failed");
                                return;
                            }
                            None => connection = Some(StreamPhase::Shutdown(stream)),
                        },
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
        }
    }
}
