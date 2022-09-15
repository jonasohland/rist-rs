pub mod processors;
use std::collections::HashMap;

use crate::{
    config::Config,
    ctl::Controller,
    processor::{Connector, ProcessorClient},
};
use processors::ProcessorHost;

use anyhow::Context;

pub struct Engine {
    processor_host: ProcessorHost,
}

impl Engine {
    pub async fn try_new(controller: Controller, config: &Config) -> anyhow::Result<Self> {
        tracing::trace!("building new engine");
        let processor_host = processors::ProcessorHost::try_new(controller, &config.proc)
            .await
            .context("failed to build processing host")?;
        Ok(Self { processor_host })
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::debug!("shutting down");
        self.processor_host.stop().await
    }

    pub fn processors(&self) -> HashMap<String, ProcessorClient> {
        self.processor_host.processors()
    }

    pub async fn connect(
        &mut self,
        destination: &str,
        name: &str,
        connector: Connector,
    ) -> anyhow::Result<()> {
        self.processor_host
            .connect(destination, name, connector)
            .await
            .with_context(|| format!("failed to connect input to {} from {}", destination, name))
    }
}
