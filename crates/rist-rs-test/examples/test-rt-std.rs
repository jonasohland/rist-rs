use std::net::SocketAddr;

use clap::Parser;
use rist_rs_std::StdRuntime;
use rist_rs_test::proto::SimpleProto;
use rist_rs_types::traits::runtime::Runtime;

#[derive(clap::Parser)]
struct Cli {
    bind: SocketAddr,

    #[clap(short, long)]
    peer: Vec<SocketAddr>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let cli = Cli::parse();
    let mut runtime = StdRuntime::try_new().unwrap();
    let socket = runtime.bind(cli.bind.into()).unwrap();
    let proto = SimpleProto::new(socket, cli.peer);
    runtime.run(proto);
}
