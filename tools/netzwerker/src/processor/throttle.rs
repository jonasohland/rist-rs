#![allow(unused)]
use super::generic::{
    connector::{simple::SimpleInput, ConnectorCollection},
    ProcessorImplementation,
};
use crate::{ctl::Controller, packet::Packet, processor::Connector};
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigPackets {
    inputs: Vec<String>,
    pps: u64,
    max_burst: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigBits {
    inputs: Vec<String>,
    bps: u64,
    max_burst: Option<u64>,
}

pub struct ConfigCommon {
    inputs: Vec<String>,
}

pub trait Config {
    fn init(&self) -> ThrottleState;
    fn common(&self) -> ConfigCommon;
}

impl Config for ConfigPackets {
    fn init(&self) -> ThrottleState {
        ThrottleState::Packets(ThrottleStatePackets { cfg: self.clone() })
    }
    fn common(&self) -> ConfigCommon {
        ConfigCommon {
            inputs: self.inputs.clone(),
        }
    }
}

impl Config for ConfigBits {
    fn init(&self) -> ThrottleState {
        ThrottleState::Bits(ThrottleStateBits { cfg: self.clone() })
    }
    fn common(&self) -> ConfigCommon {
        ConfigCommon {
            inputs: self.inputs.clone(),
        }
    }
}

pub struct ThrottleStatePackets {
    cfg: ConfigPackets,
}

pub struct ThrottleStateBits {
    cfg: ConfigBits,
}

pub enum ThrottleState {
    Packets(ThrottleStatePackets),
    Bits(ThrottleStateBits),
}

pub struct ThrottleProcessorState {
    name: String,
    input: SimpleInput,
    connectors: ConnectorCollection,
    cfg: ConfigCommon,
    throttle: ThrottleState,
}

pub enum ThrottleEvent {
    Packet(Packet),
    Wake,
}

impl ThrottleProcessorState {
    pub fn new(name: String, cfg: &impl Config) -> Self {
        Self {
            name: name.to_owned(),
            input: SimpleInput::new(&name, 1024),
            connectors: Default::default(),
            cfg: cfg.common(),
            throttle: cfg.init(),
        }
    }
}

#[async_trait::async_trait]
impl ProcessorImplementation<ThrottleEvent> for ThrottleProcessorState {
    async fn select(&mut self, _ctl: &Controller) -> Option<ThrottleEvent> {
        Some(match &mut self.throttle {
            ThrottleState::Packets(state) => state.select(&mut self.input).await,
            ThrottleState::Bits(state) => state.select(&mut self.input).await,
        })
    }

    async fn build(&mut self, ctl: &Controller) -> Result<()> {
        for input in &self.cfg.inputs {
            tracing::debug!(input, "connect input");
            ctl.connect(&self.name, input, self.input.get_connector())
                .await?
        }
        Ok(())
    }

    async fn connect(&mut self, dest: &str, label: &str, connector: Connector) -> Result<()> {
        self.connectors.connect(dest, label, connector)
    }

    async fn event(&mut self, event: ThrottleEvent) {
        match &mut self.throttle {
            ThrottleState::Packets(state) => state.process(event, &mut self.connectors).await,
            ThrottleState::Bits(state) => state.process(event, &mut self.connectors).await,
        }
    }
}

impl ThrottleStateBits {
    async fn select(&mut self, input: &mut SimpleInput) -> ThrottleEvent {
        ThrottleEvent::Packet(input.receive().await)
    }
    async fn process(&mut self, event: ThrottleEvent, connectors: &mut ConnectorCollection) {}
}

impl ThrottleStatePackets {
    async fn select(&mut self, input: &mut SimpleInput) -> ThrottleEvent {
        ThrottleEvent::Packet(input.receive().await)
    }
    async fn process(&mut self, event: ThrottleEvent, connectors: &mut ConnectorCollection) {}
}
