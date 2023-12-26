use std::io;

use rist_rs_std::StdRuntime;
use rist_rs_types::traits::{
    protocol::{Ctl, Protocol, ProtocolEvent, IOV},
    runtime::Runtime,
};

#[derive(Clone, Debug)]
struct SimpleCtl;

impl Ctl for SimpleCtl {
    type Error = io::Error;

    type Output = ();

    fn start() -> Self {
        todo!()
    }

    fn shutdown() -> Self {
        todo!()
    }
}

struct Server {}

impl Server {
    fn new() -> Self {
        Self {}
    }
}

impl<R> Protocol<R> for Server
where
    R: Runtime,
{
    type Ctl = SimpleCtl;

    fn run(&mut self, rt: &mut R, _: &[IOV<R, Self::Ctl>]) -> ProtocolEvent<R> {
        ProtocolEvent::asap(&rt.get_default_clock())
    }
}

fn main() {
    let rt = StdRuntime::try_new().unwrap();
    rt.run(Server::new());
}
