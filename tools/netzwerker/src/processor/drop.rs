use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;

use super::{generic::ProcessorImplementation, traits, Connector};
use crate::{ctl::Controller, packet::Packet};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    inputs: Vec<String>,
}

pub enum DropEvent {}

pub struct DropProcessorState {
    name: String,
    cfg: Config,
}

impl DropProcessorState {
    pub fn new(name: String, cfg: Config) -> Self {
        Self { name, cfg }
    }
}

#[async_trait]
impl ProcessorImplementation<DropEvent> for DropProcessorState {
    async fn start(&mut self, _ctl: &Controller) -> Result<()> {
        Ok(())
    }

    async fn stop(&mut self, _ctl: &Controller) -> Result<()> {
        Ok(())
    }

    async fn build(&mut self, ctl: &Controller) -> Result<()> {
        for input in &self.cfg.inputs {
            tracing::debug!(input, "connect input");
            ctl.connect(
                &self.name,
                input,
                Connector::DropConnector(DropConnector::new(input.clone())),
            )
            .await?
        }
        Ok(())
    }

    async fn connect(&mut self, _dest: &str, _label: &str, _input: Connector) -> Result<()> {
        Err(anyhow!("this processor has no outputs to connect to"))
    }

    async fn event(&mut self, _e: DropEvent) {}
}

#[derive(Debug, Clone)]
pub struct DropConnector {
    name: String,
}

impl DropConnector {
    fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl traits::ConnectorName for DropConnector {
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[async_trait]
impl traits::ConnectorCanSend for DropConnector {
    async fn can_send(&self) -> bool {
        true
    }
}

impl traits::ConnectorSendPacket for DropConnector {
    fn send_packet(&mut self, _: Packet) {}
}
