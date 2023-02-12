#![allow(unused)]
use std::{collections::HashMap, rc::Rc, task::Context};

use slab::Slab;
use tokio::net::UdpSocket;

pub struct NetSock(usize);

pub struct LocalSocket {
    socket: UdpSocket,
}

pub struct RemoteSocket {
    local_socket: Rc<LocalSocket>,
}

pub struct NetworkSockets {
    local_sockets: Slab<LocalSocket>,
    remote_sockets: HashMap<usize, UdpSocket>,
}

impl NetworkSockets {
    pub fn poll_events(&mut self, cx: &mut Context) {}
}
