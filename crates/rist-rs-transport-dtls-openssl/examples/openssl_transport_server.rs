use std::net::SocketAddr;

use clap::Parser;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[derive(Parser)]
struct Config {
    #[clap(short, long, help = "Listen on this address")]
    address: SocketAddr,

    #[clap(long, short, help = "Load server certificate from this file")]
    cert: String,

    #[clap(long, short, help = "Load server key from this file")]
    key: String,
}

fn build_acceptor(config: &Config) -> SslAcceptor {
    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::dtls()).unwrap();
    builder.set_certificate_chain_file(&config.cert).unwrap();
    builder
        .set_private_key_file(&config.key, SslFiletype::PEM)
        .unwrap();
    builder.build()
}

fn main() {
    let config = Config::parse();
    let acceptor = build_acceptor(&config);
}
