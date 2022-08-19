use std::{net::SocketAddr, ops::Deref, sync::Arc};

#[derive(Debug)]
pub struct PacketData {
    pub data: Vec<u8>,
    pub source_addr: SocketAddr,
}

impl Clone for PacketData {
    fn clone(&self) -> Self {
        tracing::warn!("packet cloned!");
        Self {
            data: self.data.clone(),
            source_addr: self.source_addr,
        }
    }
}

#[derive(Debug, Clone)]
enum PacketInner {
    Owned(PacketData),
    Shared(Arc<PacketData>),
}

impl Deref for PacketInner {
    type Target = PacketData;
    fn deref(&self) -> &PacketData {
        match self {
            PacketInner::Owned(o) => o,
            PacketInner::Shared(a) => a,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    inner: PacketInner,
}

impl Packet {
    pub fn new(from: SocketAddr, data: Vec<u8>) -> Self {
        Packet {
            inner: PacketInner::Owned(PacketData {
                data,
                source_addr: from,
            }),
        }
    }

    pub fn dup(mut self) -> (Self, Self) {
        if let PacketInner::Owned(o) = self.inner {
            self.inner = PacketInner::Shared(Arc::new(o));
        };
        (self.clone(), self)
    }
}

impl Deref for Packet {
    type Target = PacketData;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub struct PacketDuplicateStream {
    last: Option<Packet>,
}

impl PacketDuplicateStream {
    pub fn new(packet: Packet) -> Self {
        Self { last: Some(packet) }
    }

    pub fn get(&mut self) -> Packet {
        let (next, this) = self.last.take().unwrap().dup();
        self.last = Some(next);
        this
    }
}

impl Iterator for PacketDuplicateStream {
    type Item = Packet;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.get())
    }
}

impl Packet {
    pub fn into_dup_stream(self) -> PacketDuplicateStream {
        PacketDuplicateStream::new(self)
    }
}

impl IntoIterator for Packet {
    type Item = Packet;
    type IntoIter = PacketDuplicateStream;
    fn into_iter(self) -> Self::IntoIter {
        self.into_dup_stream()
    }
}

impl From<Packet> for PacketDuplicateStream {
    fn from(p: Packet) -> Self {
        Self::new(p)
    }
}
