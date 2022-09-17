use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    packet::Packet,
    processor::{traits, Connector},
};

#[derive(Clone)]
pub struct SimpleConnector {
    name: String,
    tx: Sender<Packet>,
}

pub struct SimpleInput {
    name: String,
    tx: Sender<Packet>,
    rx: Receiver<Packet>,
}

impl traits::ConnectorName for SimpleConnector {
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[async_trait::async_trait]
impl traits::ConnectorCanSend for SimpleConnector {
    async fn can_send(&self) -> bool {
        !self.tx.is_closed()
    }
}

impl traits::ConnectorSendPacket for SimpleConnector {
    fn send_packet(&mut self, packet: Packet) {
        self.tx.try_send(packet).ok();
    }
}

impl SimpleInput {
    pub fn new(name: &str, buffer: usize) -> Self {
        let (tx, rx) = channel(buffer);
        Self {
            name: name.to_owned(),
            tx,
            rx,
        }
    }

    pub fn get_connector(&self) -> Connector {
        Connector::SimpleConnector(SimpleConnector {
            name: self.name.clone(),
            tx: self.tx.clone(),
        })
    }

    pub async fn receive(&mut self) -> Packet {
        // will always return a packet (or get cancelled) because a sender is stored in this
        // struct
        self.rx.recv().await.unwrap()
    }
}
