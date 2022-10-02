use rist_rs_runtime_std::transport::socket::NonBlockingUdpSocket;
use rist_rs_transport_dtls_openssl::transport::stream::non_blocking::DtlsStreamAcceptor;
use rist_rs_transport_dtls_openssl::transport::stream::{self, SimpleContextProvider};

use std::collections::LinkedList;
use std::fs;
use std::net::SocketAddr;

use clap::Parser;

use openssl::pkey;
use openssl::x509;

#[derive(Parser)]
struct Config {
    #[clap(short, long, help = "Listen on this address")]
    address: SocketAddr,

    #[clap(long, short, help = "Load server certificate from this file")]
    cert: String,

    #[clap(long, short, help = "Load server key from this file")]
    key: String,

    #[clap(long, short, help = "Ca certificate")]
    ca: Option<String>,

    #[clap(long, short, help = "Verify the client certificate")]
    verify_client_cert: bool,

    #[clap(long, short, help = "Set client hostname that should be verified")]
    client_hostname: Option<String>,
}

fn load_stream_config(config: &Config) -> stream::Config<SimpleContextProvider> {
    let mut builder = SimpleContextProvider::builder()
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
    if config.verify_client_cert {
        builder = builder.with_verify_client_cert()
    }
    if let Some(name) = &config.client_hostname {
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

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    let config = Config::parse();
    let mut acceptor = DtlsStreamAcceptor::new(
        NonBlockingUdpSocket::bind(config.address).unwrap(),
        1524,
        load_stream_config(&config),
    );
    tracing::debug!(address = ?config.address, "listening");
    let mut streams = LinkedList::new();
    loop {
        match acceptor.try_accept() {
            Some(res) => match res {
                Ok(new_streams) => new_streams.into_iter().for_each(|s| streams.push_back(s)),
                Err(err) => {
                    tracing::error!(error = ?err, "accept failed")
                }
            },
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
        streams = streams
            .into_iter()
            .filter_map(|mut stream| match stream.shutdown() {
                Some(result) => match result {
                    Ok(_) => None,
                    Err(error) => {
                        tracing::warn!(?error, "stream shutdown failed");
                        None
                    }
                },
                None => Some(stream),
            })
            .collect();
    }
}
