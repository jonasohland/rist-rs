use std::{collections::VecDeque, ops::Add, time::Duration};

use super::generic::{
    connector::{simple::SimpleInput, ConnectorCollection},
    ProcessorImplementation,
};
use crate::{ctl::Controller, packet::Packet, processor::Connector};
use anyhow::Result;
use serde::Deserialize;
use tokio::{select, time::Instant};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    inputs: Vec<String>,
    delay: u64,
}

pub struct DelayedPacket {
    packet: Packet,
    deadline: Instant,
}

impl DelayedPacket {
    fn after(packet: Packet, duration: Duration) -> DelayedPacket {
        DelayedPacket {
            packet,
            deadline: Instant::now().add(duration),
        }
    }
}

pub enum DelayEvent {
    Packet(DelayedPacket),
    WakeUp,
}

pub struct DelayProcessorState {
    cfg: Config,
    name: String,
    input: SimpleInput,
    connectors: ConnectorCollection,
    queue: VecDeque<DelayedPacket>,
    next_deadline: Option<Instant>,
}

impl DelayProcessorState {
    pub fn new(name: String, cfg: &Config) -> Self {
        Self {
            cfg: cfg.clone(),
            name: name.to_owned(),
            input: SimpleInput::new(&name, 1024),
            connectors: Default::default(),
            queue: Default::default(),
            next_deadline: None,
        }
    }
}

#[async_trait::async_trait]
impl ProcessorImplementation<DelayEvent> for DelayProcessorState {
    async fn select(&mut self, _ctl: &Controller) -> Option<DelayEvent> {
        match self.next_deadline.take() {
            Some(instant) => {
                if instant <= Instant::now() {
                    Some(DelayEvent::WakeUp)
                } else {
                    let sleep = tokio::time::sleep_until(instant);
                    tokio::pin!(sleep);
                    select! {
                        _ = &mut sleep => {
                            Some(DelayEvent::WakeUp)
                        },
                        packet = self.input.receive() => {
                            Some(DelayEvent::Packet(DelayedPacket::after(
                                packet,
                                Duration::from_millis(self.cfg.delay)
                            )))
                        }
                    }
                }
            }
            None => Some(DelayEvent::Packet(DelayedPacket::after(
                self.input.receive().await,
                Duration::from_millis(self.cfg.delay),
            ))),
        }
    }

    async fn start(&mut self, _ctl: &Controller) -> Result<()> {
        Ok(())
    }

    async fn stop(&mut self, _ctl: &Controller) -> Result<()> {
        Ok(())
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

    async fn event(&mut self, e: DelayEvent) {
        match e {
            DelayEvent::Packet(packet) => {
                self.packet(packet).await;
                self.send_next().await;
            }
            DelayEvent::WakeUp => self.send_next().await,
        }
        self.schedule_next_packet();
    }
}

impl DelayProcessorState {
    async fn send(&mut self, packet: DelayedPacket) {
        self.connectors.send(packet.packet).await
    }

    async fn send_next(&mut self) {
        while self
            .queue
            .front()
            .map(|p| p.deadline <= Instant::now())
            .unwrap_or(false)
        {
            let p = self.queue.pop_front().unwrap();
            self.send(p).await;
        }
    }

    async fn packet(&mut self, packet: DelayedPacket) {
        let now = Instant::now();
        if packet.deadline <= now {
            self.send_next().await;
            self.send(packet).await;
        } else {
            self.queue.push_back(packet)
        }
    }

    fn schedule_next_packet(&mut self) {
        if let Some(deadline) = self.queue.front().map(|p| p.deadline) {
            self.next_deadline = Some(deadline)
        }
    }
}
