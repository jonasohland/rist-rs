use std::collections::HashMap;

use crate::{engine::processors::DEFAULT_OUTPUT_LABEL, packet::Packet, processor::Connector, util};

use anyhow::{anyhow, Result};

pub mod simple;

#[derive(Default)]
pub struct ConnectorCollection {
    connectors: Vec<Connector>,
}

impl ConnectorCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn connect(&mut self, dest: &str, label: &str, connector: Connector) -> Result<()> {
        if label != DEFAULT_OUTPUT_LABEL {
            Err(anyhow!(
                "no output labelled '{}' found for this processor",
                label
            ))
        } else {
            tracing::info!(
                "connected '{}' to destination '{}'",
                DEFAULT_OUTPUT_LABEL,
                dest
            );
            self.connectors.push(connector);
            Ok(())
        }
    }

    pub async fn send(&mut self, packet: Packet) {
        util::send_packet_to(&mut self.connectors, packet)
    }
}

pub struct NamedCollectorCollection {
    collectors: HashMap<String, Vec<Connector>>,
}

impl NamedCollectorCollection {
    pub fn new<T>(names: impl IntoIterator<Item = T>) -> Self
    where
        T: ToString,
    {
        Self {
            collectors: names
                .into_iter()
                .map(|i| (i.to_string(), Vec::new()))
                .collect(),
        }
    }

    pub fn connect(&mut self, dest: &str, label: &str, connector: Connector) -> Result<()> {
        match self.collectors.get_mut(label) {
            None => Err(anyhow!("no output labelled '{}' found", label)),
            Some(collection) => {
                tracing::info!("connected '{}' to destination '{}'", label, dest);
                collection.push(connector);
                Ok(())
            }
        }
    }

    pub fn send(&mut self, label: &str, packet: Packet) -> Result<()> {
        match self.collectors.get_mut(label) {
            None => Err(anyhow!(
                "no output labelled '{}' found for this processor",
                label
            )),
            Some(collection) => {
                util::send_packet_to(collection, packet);
                Ok(())
            }
        }
    }
}
