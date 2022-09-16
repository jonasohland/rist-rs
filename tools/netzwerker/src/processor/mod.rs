pub mod delay;
pub mod drop;
pub mod generic;
pub mod rx;
pub mod splitter;
pub mod traits;
pub mod tx;

use drop::DropConnector;
use generic::{connector::simple::SimpleConnector, GenericProcessor, GenericProcessorClient};
use rx::{RxProcessor, RxProcessorClient};
use splitter::{SplitterProcessor, SplitterProcessorClient};
use tx::{TxProcessor, TxProcessorClient};

use crate::{packet::Packet, util::dispatch_enum::dispatch_enum};
use anyhow::{Error, Result};
use async_trait::async_trait;
use std::fmt::Debug;

dispatch_enum! {
    (ProcessorClient, Clone) { RxProcessorClient, TxProcessorClient, SplitterProcessorClient, GenericProcessorClient },
    (Connector, Clone) { SimpleConnector, DropConnector },
    (Processor) { RxProcessor, TxProcessor, SplitterProcessor, GenericProcessor }
}

dispatch_enum! {
    (Processor: traits::ProcessorJoin) => {
        async join(mut self) -> Result<()> { RxProcessor, TxProcessor, SplitterProcessor, GenericProcessor }
    }
}

dispatch_enum! {
    (Processor: traits::ProcessorGetClient) => {
        client() -> ProcessorClient { RxProcessor, TxProcessor, SplitterProcessor, GenericProcessor }
    }
}

dispatch_enum! {
    (ProcessorClient: traits::ProcessorClientLifecycle) => {
        async build() -> Result<()> { RxProcessorClient, TxProcessorClient, SplitterProcessorClient, GenericProcessorClient },
        async start() -> Result<()> { RxProcessorClient, TxProcessorClient, SplitterProcessorClient, GenericProcessorClient },
        async stop() -> Result<()> { RxProcessorClient, TxProcessorClient, SplitterProcessorClient, GenericProcessorClient }
    }
}

dispatch_enum! {
    (ProcessorClient: traits::ProcessorClientConnectInput) => {
        async connect(destination: &str, name: &str, connector: Connector) -> Result<()> { RxProcessorClient, TxProcessorClient, SplitterProcessorClient, GenericProcessorClient }
    }
}

dispatch_enum! {
    (Connector: traits::ConnectorName) => {
        name() -> String { SimpleConnector, DropConnector }
    }
}

dispatch_enum! {
    (Connector: traits::ConnectorCanSend) => {
        async can_send() -> bool { SimpleConnector, DropConnector }
    }
}

dispatch_enum! {
    (Connector: traits::ConnectorSendPacket) => {
        send_packet(&mut self, packet: Packet) -> () { SimpleConnector, DropConnector }
    }
}

impl Debug for Connector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessorInput").finish()
    }
}
