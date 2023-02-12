use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    time::Duration,
};

use rand::Rng;

pub fn limit_tries<T>(limit: usize, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    for _ in 0..limit {
        if let Some(res) = f() {
            return Some(res);
        }
    }
    None
}

pub fn get_localhost_bound_socket() -> (u16, UdpSocket) {
    limit_tries(10000, || {
        let port = rand::thread_rng().gen_range(10000..40000);
        UdpSocket::bind(sock_addr_localhost(port))
            .ok()
            .map(|sock| (port, sock))
    })
    .expect("failed to find a free UDP port")
}

pub fn sock_addr_localhost(port: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
}

pub struct BusyLoopTimeout(u128, u128);

impl BusyLoopTimeout {
    pub fn new(timeout: Duration) -> Self {
        Self(0, timeout.as_millis() / 10)
    }

    pub fn sleep(&mut self) -> bool {
        if self.0 > self.1 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
        self.0 += 1;
        false
    }
}
