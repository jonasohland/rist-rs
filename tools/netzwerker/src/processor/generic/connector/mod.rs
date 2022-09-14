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
