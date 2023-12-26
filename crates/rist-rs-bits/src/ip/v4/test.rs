use super::*;
use core::str::FromStr;

const IP_BASIC: [u8; 52] = [
    0x45, 0x00, 0x00, 0x34, 0x61, 0x5f, 0x40, 0x00, 0x30, 0x06, 0xd2, 0xa3, 0x54, 0xf6, 0xfb, 0xc9,
    0xc0, 0xa8, 0x05, 0x59, 0x01, 0xbb, 0xff, 0x15, 0xe3, 0xa0, 0xbb, 0xfc, 0xb9, 0x0d, 0x82, 0x05,
    0x80, 0x11, 0x01, 0x54, 0x49, 0xe2, 0x00, 0x00, 0x01, 0x01, 0x08, 0x0a, 0xed, 0xe5, 0xf8, 0x7a,
    0x62, 0xed, 0xef, 0xf4,
];

fn packet(data: &[u8]) -> Ipv4PacketView {
    Ipv4PacketView::try_from(data).unwrap()
}

#[test]
fn basic() {
    let ip = packet(&IP_BASIC);
    assert_eq!(ip.header_len(), 20);
    assert_eq!(ip.dscp(), 0);
    assert_eq!(ip.ecn(), 0);
    assert_eq!(ip.ttl(), 48);
    assert_eq!(ip.checksum(), 0xd2a3);
    assert!(ip.df());
    assert!(!ip.mf());
    assert!(!ip.is_fragmented());
    assert_eq!(ip.protocol(), 6);
    assert_eq!(
        std::net::IpAddr::from_str("84.246.251.201").unwrap(),
        std::net::IpAddr::from(ip.source_addr())
    );
    assert_eq!(
        std::net::IpAddr::from_str("192.168.5.89").unwrap(),
        std::net::IpAddr::from(ip.dest_addr())
    );
}

/// Packet #0 from a fragmented Ipv4 Packet
const FRAG_0: [u8; 28] = [
    0x45, 0x00, 0x00, 0x1c, 0xf5, 0xaf, 0x20, 0x00, 0x40, 0x11, 0x00, 0x00, 0x83, 0xb3, 0xc4, 0xdc,
    0x83, 0xb3, 0xc4, 0x2e, // payload
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Packet #1 from a fragmented Ipv4 Packet
const FRAG_1: [u8; 28] = [
    0x45, 0x00, 0x00, 0x1c, 0xf5, 0xaf, 0x20, 0x01, 0x40, 0x11, 0x00, 0x00, 0x83, 0xb3, 0xc4, 0xdc,
    0x83, 0xb3, 0xc4, 0x2e, // payload
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Packet #2 from a fragmented Ipv4 Packet
const FRAG_2: [u8; 28] = [
    0x45, 0x00, 0x00, 0x1c, 0xf5, 0xaf, 0x20, 0x02, 0x40, 0x11, 0x00, 0x00, 0x83, 0xb3, 0xc4, 0xdc,
    0x83, 0xb3, 0xc4, 0x2e, // payload
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Packet #3 from a fragmented Ipv4 Packet
const FRAG_3: [u8; 28] = [
    0x45, 0x00, 0x00, 0x1c, 0xf5, 0xaf, 0x00, 0x03, 0x40, 0x11, 0x00, 0x00, 0x83, 0xb3, 0xc4, 0xdc,
    0x83, 0xb3, 0xc4, 0x2e, // payload
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[test]
fn fragmentation() {
    let fragments = [
        packet(&FRAG_0),
        packet(&FRAG_1),
        packet(&FRAG_2),
        packet(&FRAG_3),
    ];

    assert!(fragments[0].mf());
    assert!(fragments[1].mf());
    assert!(fragments[2].mf());
    assert!(!fragments[3].mf());

    assert!(!fragments[0].df());
    assert!(!fragments[1].df());
    assert!(!fragments[2].df());
    assert!(!fragments[3].df());

    assert_eq!(fragments[0].offset(), 0);
    assert_eq!(fragments[1].offset(), fragments[0].payload().unwrap().len());
    assert_eq!(
        fragments[2].offset(),
        fragments[0].payload().unwrap().len() + fragments[1].payload().unwrap().len()
    );
    assert_eq!(
        fragments[3].offset(),
        fragments[0].payload().unwrap().len()
            + fragments[1].payload().unwrap().len()
            + fragments[2].payload().unwrap().len()
    );

    assert_eq!(fragments[0].identification(), 0xf5af);
    assert_eq!(fragments[1].identification(), 0xf5af);
    assert_eq!(fragments[2].identification(), 0xf5af);
    assert_eq!(fragments[3].identification(), 0xf5af);

    assert!(fragments[0].is_fragmented());
    assert!(fragments[1].is_fragmented());
    assert!(fragments[2].is_fragmented());
    assert!(fragments[3].is_fragmented())
}

const MULTICAST: [u8; 21] = [
    0x45, 0x00, 0x00, 0x15, 0xf8, 0xe2, 0x00, 0x00, 0xff, 0x11, 0x1b, 0xda, 0xc0, 0xa8, 0x05, 0x43,
    0xe0, 0x00, 0x00, 0xfb, 0xff,
];

#[test]
fn multicast() {
    let ip = packet(&MULTICAST);
    assert!(ip.is_multicast());
    assert_eq!(ip.payload().unwrap(), &[0xff_u8])
}

#[test]
fn broken_header_len() {
    // packet with invalid ihl field (4)
    let err = Ipv4PacketView::try_from(
        [
            0x41, 0x00, 0x00, 0x53, 0x2e, 0xb5, 0x00, 0x00, 0x79, 0x06, 0x49, 0x05, 0x23, 0xba,
            0xe0, 0x2f, 0xc0, 0xa8, 0x05, 0x59, 0xff,
        ]
        .as_slice(),
    )
    .unwrap_err();
    if let super::super::error::Error::V4(e) = err {
        match e.kind() {
            error::ErrorKind::NotEnoughData { need, got, field } => {
                assert_eq!(need, 20);
                assert_eq!(got, 4);
            }
            _ => panic!("wrong error kind returned"),
        }
    } else {
        panic!("wrong error type returned")
    }
}

#[test]
fn broken_total_len() {
    // packet with legal header but broken total len field
    let ip = packet(&[
        0x45, 0x00, 0x00, 0x00, 0x2e, 0xb5, 0x00, 0x00, 0x79, 0x06, 0x49, 0x05, 0x23, 0xba, 0xe0,
        0x2f, 0xc0, 0xa8, 0x05, 0x59, 0xff,
    ]);

    // broken total length -> no payload
    assert!(ip.payload().is_err());
    // options still valid because header is valid
    assert!(ip.options().is_ok());
}
