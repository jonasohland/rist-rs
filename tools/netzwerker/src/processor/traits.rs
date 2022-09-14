use super::Result;
use crate::packet::Packet;
use async_trait::async_trait;

#[async_trait]
pub trait ConnectorName {
    fn name(&self) -> String;
}

#[async_trait]
pub trait ConnectorCanSend {
    async fn can_send(&self) -> bool;
}

#[async_trait]
pub trait ConnectorSendPacket {
    fn send_packet(&mut self, packet: Packet);
}

#[async_trait]
pub trait ProcessorJoin {
    async fn join(self) -> Result<()>;
}

pub trait ProcessorGetClient {
    fn client(&self) -> super::ProcessorClient;
}

#[async_trait]
pub trait ProcessorClientLifecycle {
    async fn build(&self) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

#[async_trait]
pub trait ProcessorClientConnectInput {
    async fn connect(
        &self,
        destination: &str,
        name: &str,
        input: super::Connector,
    ) -> Result<()>;
}