#![allow(unused)]
use core::fmt::Display;
use rist_rs_macros::features::cfg_std;

/// An Ipv6 Address
#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Ord, Clone, Copy)]
pub struct Ipv6Addr {
    data: [u8; 16],
}

impl From<[u8; 16]> for Ipv6Addr {
    fn from(data: [u8; 16]) -> Self {
        Self { data }
    }
}

impl From<[u16; 8]> for Ipv6Addr {
    fn from(data: [u16; 8]) -> Self {
        Self::new(
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        )
    }
}

impl From<u128> for Ipv6Addr {
    fn from(v: u128) -> Self {
        Ipv6Addr::from(v.to_be_bytes())
    }
}

impl Ipv6Addr {
    #[allow(clippy::too_many_arguments)]
    fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        // always safe to transmute [u16; 8] to [u8; 16]
        unsafe {
            Self {
                data: core::mem::transmute([
                    a.to_be(),
                    b.to_be(),
                    c.to_be(),
                    d.to_be(),
                    e.to_be(),
                    f.to_be(),
                    g.to_be(),
                    h.to_be(),
                ]),
            }
        }
    }

    fn segments(&self) -> [u16; 8] {
        let [a, b, c, d, e, f, g, h] = unsafe { core::mem::transmute::<_, [u16; 8]>(self.data) };
        [
            u16::from_be(a),
            u16::from_be(b),
            u16::from_be(c),
            u16::from_be(d),
            u16::from_be(e),
            u16::from_be(f),
            u16::from_be(g),
            u16::from_be(h),
        ]
    }

    fn octets(&self) -> [u8; 16] {
        self.data
    }

    fn as_int(&self) -> u128 {
        u128::from_be_bytes(self.data)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Ord, Clone, Copy)]
pub struct Ipv4Addr {
    data: [u8; 4],
}

impl From<[u8; 4]> for Ipv4Addr {
    fn from(data: [u8; 4]) -> Self {
        Self { data }
    }
}

impl From<u32> for Ipv4Addr {
    fn from(v: u32) -> Self {
        Self::from(v.to_be_bytes())
    }
}

impl Ipv4Addr {
    const fn new(o0: u8, o1: u8, o2: u8, o3: u8) -> Self {
        Self {
            data: [o0, o1, o2, o3],
        }
    }

    pub const fn is_unspecified(&self) -> bool {
        self.as_int() == 0
    }

    pub const fn is_loopback(&self) -> bool {
        self.data[0] == 127
    }

    pub const fn is_private(&self) -> bool {
        match self.data {
            [10, ..] => true,
            [172, b, ..] if b >= 16 && b <= 31 => true,
            [192, 168, ..] => true,
            _ => false,
        }
    }

    pub const fn is_link_local(&self) -> bool {
        matches!(self.octets(), [169, 254, ..])
    }

    pub const fn is_multicast(&self) -> bool {
        self.octets()[0] >= 224 && self.octets()[0] <= 239
    }

    pub const fn octets(&self) -> [u8; 4] {
        self.data
    }

    pub const fn as_int(&self) -> u32 {
        u32::from_be_bytes(self.data)
    }
}

impl Display for Ipv4Addr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.data[0], self.data[1], self.data[2], self.data[3]
        )
    }
}

pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl From<[u8; 4]> for IpAddr {
    fn from(data: [u8; 4]) -> Self {
        IpAddr::V4(Ipv4Addr { data })
    }
}

impl From<u32> for IpAddr {
    fn from(v: u32) -> Self {
        IpAddr::V4(Ipv4Addr::from(v.to_be_bytes()))
    }
}

impl From<Ipv4Addr> for IpAddr {
    fn from(addr: Ipv4Addr) -> Self {
        IpAddr::V4(addr)
    }
}

impl From<Ipv6Addr> for IpAddr {
    fn from(addr: Ipv6Addr) -> Self {
        IpAddr::V6(addr)
    }
}

cfg_std! {

    impl From<Ipv4Addr> for std::net::Ipv4Addr {
        fn from(a: Ipv4Addr) -> Self {
            std::net::Ipv4Addr::from(a.octets())
        }
    }

    impl From<Ipv4Addr> for std::net::IpAddr {
        fn from(a: Ipv4Addr) -> Self {
            IpAddr::from(a).into()
        }
    }

    impl From<Ipv6Addr> for std::net::IpAddr {
        fn from(a: Ipv6Addr) -> Self {
            IpAddr::from(a).into()
        }
    }

    impl From<Ipv6Addr> for std::net::Ipv6Addr {
        fn from(a: Ipv6Addr) -> Self {
            std::net::Ipv6Addr::from(a.octets())
        }
    }

    impl From<IpAddr> for std::net::IpAddr {
        fn from(a: IpAddr) -> Self {
            match a {
                IpAddr::V4(v4) => std::net::IpAddr::V4(v4.into()),
                IpAddr::V6(v6) => std::net::IpAddr::V6(v6.into()),
            }
        }
    }

}
