use std::{collections::HashMap, fmt::Display};

use tracing::Instrument;

use crate::{
    config::ProcessorConfigs,
    ctl::Controller,
    processor::{
        generic,
        drop::DropProcessorState,
        rx, splitter,
        traits::{
            ProcessorClientConnectInput, ProcessorClientLifecycle, ProcessorGetClient,
            ProcessorJoin,
        },
        tx, Connector, Processor, ProcessorClient,
    },
};

use anyhow::{anyhow, Context, Error, Result};

pub const DEFAULT_OUTPUT_LABEL: &str = "main";

pub struct ProcessorHost {
    processors: HashMap<String, Processor>,
}

async fn build_processor(
    controller: Controller,
    name: String,
    config: &ProcessorConfigs,
) -> anyhow::Result<(String, Processor)> {
    tracing::trace!(name, ?config, "build new processor");
    Ok((
        name.to_owned(),
        match config {
            ProcessorConfigs::Rx(cfg) => Processor::RxProcessor(
                rx::RxProcessor::try_new(name.to_owned(), cfg, controller)
                    .await
                    .context(format!("failed to build rx processor: {}", name))?,
            ),
            ProcessorConfigs::Splitter(cfg) => Processor::SplitterProcessor(
                splitter::SplitterProcessor::try_new(name.to_owned(), cfg, controller)
                    .await
                    .context(format!("failed to build splitter processor {}", name))?,
            ),
            ProcessorConfigs::Tx(cfg) => Processor::TxProcessor(
                tx::TxProcessor::try_new(name.to_owned(), cfg, controller)
                    .await
                    .with_context(|| format!("failed to build tx processor {}", name))?,
            ),
            ProcessorConfigs::Drop(cfg) => generic::generic_processor!(
                "drop", name.clone(), controller, DropProcessorState::new(name.to_owned(), cfg.clone())
            ),
        },
    ))
}

struct ProcErr(String, Error);

impl Display for ProcErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.0, self.1)
    }
}

fn vec_to_context(vec: Vec<ProcErr>, err: Error) -> Error {
    vec.into_iter().fold(err, |e, next| e.context(next))
}

impl ProcessorHost {
    pub async fn try_new(
        controller: Controller,
        configs: &HashMap<String, ProcessorConfigs>,
    ) -> anyhow::Result<Self> {
        tracing::trace!("building new processor host");
        Ok(Self {
            processors: futures::future::join_all(configs.iter().map(|(name, cfg)| {
                build_processor(controller.clone(), name.to_owned(), cfg).instrument(tracing::span!(
                    tracing::Level::ERROR,
                    "proc",
                    ins = name
                ))
            }))
            .await
            .into_iter()
            .collect::<Result<HashMap<_, _>>>()?,
        })
    }

    pub async fn stop(&mut self) -> Result<()> {
        let errs =
            futures::future::join_all(self.processors.drain().map(|(name, proc)| async move {
                tracing::trace!(name, "stopping processor");
                proc.client()
                    .stop()
                    .await
                    .map_err(|e| ProcErr(name.clone(), e))?;
                proc.join().await.map_err(|e| ProcErr(name, e))
            }))
            .await
            .into_iter()
            .filter_map(|e| e.err())
            .collect::<Vec<_>>();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(vec_to_context(
                errs,
                anyhow::anyhow!("not all processors could be stopped"),
            ))
        }
    }

    pub async fn connect(&self, destination: &str, name: &str, connector: Connector) -> Result<()> {
        let (proc_name, label) = name
            .split_once('.')
            .map(|(l, r)| (l.to_owned(), r.to_owned()))
            .unwrap_or_else(|| (name.to_owned(), DEFAULT_OUTPUT_LABEL.to_owned()));
        self.processors
            .get(&proc_name)
            .ok_or_else(|| anyhow!("processor {} not found", proc_name))?
            .client()
            .connect(destination, &label, connector)
            .await
    }

    pub fn processors(&self) -> HashMap<String, ProcessorClient> {
        self.processors
            .iter()
            .map(|(s, p)| (s.clone(), p.client()))
            .collect()
    }
}
