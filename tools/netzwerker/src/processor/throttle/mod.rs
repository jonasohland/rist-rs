use self::credit::CreditCounter;
use super::generic::{
    connector::{simple::SimpleInput, NamedCollectorCollection},
    ProcessorImplementation,
};
use crate::{
    ctl::Controller, engine::processors::DEFAULT_OUTPUT_LABEL, packet::Packet, processor::Connector,
};
use anyhow::Result;
use rist_rs_core::collections::static_vec_deque::StaticVecDeque;
use serde::Deserialize;
use std::time::Duration;
use tokio::{select, time::sleep};

mod credit;

const MAX_BURST_DEFAULT: u64 = 1000;

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigPackets {
    inputs: Vec<String>,
    pps: u64,
    size: u64,
    max_burst: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigBits {
    inputs: Vec<String>,
    bps: u64,
    size: u64,
    max_burst: Option<u64>,
}

pub struct ConfigCommon {
    inputs: Vec<String>,
    max_burst: u64,
}

pub trait Config {
    fn init(&self) -> ThrottleState;
    fn common(&self) -> ConfigCommon;
}

impl Config for ConfigPackets {
    fn init(&self) -> ThrottleState {
        ThrottleState::Packets(ThrottleStatePackets {
            queue: StaticVecDeque::new(self.common().max_burst as usize),
            credit_counter: CreditCounter::new(self.pps, self.size),
            next_packet_in: None,
        })
    }
    fn common(&self) -> ConfigCommon {
        ConfigCommon {
            inputs: self.inputs.clone(),
            max_burst: self.max_burst.unwrap_or(MAX_BURST_DEFAULT),
        }
    }
}

impl Config for ConfigBits {
    fn init(&self) -> ThrottleState {
        ThrottleState::Bits(ThrottleStateBits {
            queue: StaticVecDeque::new(self.common().max_burst as usize),
            credit_counter: CreditCounter::new(self.bps, self.size),
            next_packet_in: None,
        })
    }
    fn common(&self) -> ConfigCommon {
        ConfigCommon {
            inputs: self.inputs.clone(),
            max_burst: self.max_burst.unwrap_or(MAX_BURST_DEFAULT),
        }
    }
}

pub struct ThrottleStatePackets {
    queue: StaticVecDeque<Packet>,
    credit_counter: CreditCounter,
    next_packet_in: Option<Duration>,
}

pub struct ThrottleStateBits {
    queue: StaticVecDeque<Packet>,
    credit_counter: CreditCounter,
    next_packet_in: Option<Duration>,
}

pub enum ThrottleState {
    Packets(ThrottleStatePackets),
    Bits(ThrottleStateBits),
}

pub struct ThrottleProcessorState {
    name: String,
    input: SimpleInput,
    connectors: NamedCollectorCollection,
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
            connectors: NamedCollectorCollection::new(["drop", DEFAULT_OUTPUT_LABEL]),
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
            ThrottleState::Packets(state) => state.process(event, &mut self.connectors),
            ThrottleState::Bits(state) => state.process(event, &mut self.connectors),
        }
    }
}

impl ThrottleStateBits {
    async fn select(&mut self, input: &mut SimpleInput) -> ThrottleEvent {
        match self.next_packet_in.take() {
            Some(duration) => select! {
                packet = input.receive() => ThrottleEvent::Packet(packet),
                _ = sleep(duration) => ThrottleEvent::Wake
            },
            None => ThrottleEvent::Packet(input.receive().await),
        }
    }

    fn enqueue(&mut self, packet: Packet, connectors: &mut NamedCollectorCollection) {
        if let Some(rejected) = self.queue.push_back(packet) {
            connectors.send("drop", rejected).unwrap()
        }
    }

    fn dequeue_and_send(&mut self, connectors: &mut NamedCollectorCollection) {
        while !self.queue.is_empty() {
            match self
                .credit_counter
                .take(self.queue.front().unwrap().data.len() as u64 * 8)
            {
                Some(_) => connectors
                    .send(DEFAULT_OUTPUT_LABEL, self.queue.pop_front().unwrap())
                    .unwrap(),
                None => break,
            };
        }
    }

    fn process(&mut self, event: ThrottleEvent, connectors: &mut NamedCollectorCollection) {
        self.credit_counter.update();
        if let ThrottleEvent::Packet(packet) = event {
            self.enqueue(packet, connectors);
        }
        self.dequeue_and_send(connectors);
        while !self.queue.is_empty() {
            match self
                .credit_counter
                .sleep_time_to_availability(self.queue.front().unwrap().data.len() as u64 * 8)
            {
                Ok(duration) => {
                    self.next_packet_in = Some(duration);
                    break;
                }
                Err(e) => {
                    self.queue.pop_front().unwrap();
                    tracing::error!("dropping packet: {}", e);
                }
            }
        }
    }
}

impl ThrottleStatePackets {
    async fn select(&mut self, input: &mut SimpleInput) -> ThrottleEvent {
        match self.next_packet_in.take() {
            Some(duration) => select! {
                packet = input.receive() => ThrottleEvent::Packet(packet),
                _ = sleep(duration) => ThrottleEvent::Wake
            },
            None => ThrottleEvent::Packet(input.receive().await),
        }
    }

    fn enqueue(&mut self, packet: Packet, connectors: &mut NamedCollectorCollection) {
        if let Some(rejected) = self.queue.push_back(packet) {
            connectors.send("drop", rejected).unwrap()
        }
    }

    fn dequeue_and_send(&mut self, connectors: &mut NamedCollectorCollection) {
        while !self.queue.is_empty() {
            match self.credit_counter.take(1) {
                Some(_) => connectors
                    .send(DEFAULT_OUTPUT_LABEL, self.queue.pop_front().unwrap())
                    .unwrap(),
                None => break,
            };
        }
    }

    fn process(&mut self, event: ThrottleEvent, connectors: &mut NamedCollectorCollection) {
        self.credit_counter.update();
        if let ThrottleEvent::Packet(packet) = event {
            self.enqueue(packet, connectors);
        }
        self.dequeue_and_send(connectors);
        if !self.queue.is_empty() {
            self.next_packet_in = Some(self.credit_counter.sleep_time_to_availability(1).unwrap())
        }
    }
}
