use super::generic::connector::simple::SimpleInput;
use super::{traits, Connector};
use crate::error::SendErrorWithContext;
use crate::util::{self, processor_tracing_scope};
use crate::{ctl::Controller, packet::Packet};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Future;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Range;
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    sync::oneshot,
    task::JoinHandle,
};
use tracing::Instrument;

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    inputs: Vec<String>,
    outputs: HashMap<String, OutputConfig>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct OutputConfig {
    percent: Option<f32>,
}

impl OutputConfig {
    fn bound(&self, from: f32) -> Option<Range<f32>> {
        self.percent.map(|v| from..from + (v / 100.0))
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

pub struct SplitterProcessor {
    tx: UnboundedSender<ProcessorEvent>,
    task: JoinHandle<()>,
}

#[async_trait]
impl traits::ProcessorJoin for SplitterProcessor {
    async fn join(mut self) -> super::Result<()> {
        Ok(self.task.await?)
    }
}

impl traits::ProcessorGetClient for SplitterProcessor {
    fn client(&self) -> super::ProcessorClient {
        super::ProcessorClient::SplitterProcessorClient(SplitterProcessorClient {
            tx: self.tx.clone(),
        })
    }
}

impl SplitterProcessor {
    pub async fn try_new(name: String, config: &Config, controller: Controller) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let (outputs, fallbacks) = build_outputs(&config.outputs)
            .context("failed to build processor outputs from configuration")?;
        Ok(Self {
            tx,
            task: tokio::task::spawn(ProcessorState::launch(
                name,
                config.clone(),
                controller,
                outputs,
                fallbacks,
                rx,
            )),
        })
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Clone)]
pub struct SplitterProcessorClient {
    tx: UnboundedSender<ProcessorEvent>,
}

#[async_trait]
impl traits::ProcessorClientLifecycle for SplitterProcessorClient {
    async fn build(&self) -> super::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ProcessorEvent::Build(tx))
            .send_err_ctx("Build", "SplitterProcessor")?;
        rx.await?
    }

    async fn start(&self) -> super::Result<()> {
        self.tx
            .send(ProcessorEvent::Start)
            .send_err_ctx("Start", "SplitterProcessor")
    }

    async fn stop(&self) -> super::Result<()> {
        self.tx.send(ProcessorEvent::Stop)?;
        Ok(())
    }
}

#[async_trait]
impl traits::ProcessorClientConnectInput for SplitterProcessorClient {
    async fn connect(
        &self,
        destination: &str,
        label: &str,
        input: super::Connector,
    ) -> super::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ProcessorEvent::ConnectInput(
                destination.to_owned(),
                label.to_owned(),
                input,
                tx,
            ))
            .send_err_ctx("ConnectInput", "SplitterProcessor")?;
        rx.await?
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Debug)]
struct NormalOutput {
    label: String,
    bounds: Range<f32>,
    connectors: Vec<Connector>,
}

impl NormalOutput {
    fn select(&self, r: f32) -> bool {
        self.bounds.contains(&r)
    }
}

#[derive(Debug)]
struct FallbackOutput {
    label: String,
    connectors: Vec<Connector>,
}

#[derive(Debug)]
enum Output {
    Normal(NormalOutput),
    Fallback(FallbackOutput),
}

impl Output {
    fn new_fallback(label: &str) -> Self {
        Output::Fallback(FallbackOutput {
            label: label.to_owned(),
            connectors: vec![],
        })
    }

    fn try_make_next(inc: f32, label: &str, config: &OutputConfig) -> Result<(Self, f32)> {
        config
            .bound(inc)
            .map(|bounds| {
                if bounds.end >= 1f32 {
                    Err(anyhow!("out of range"))
                } else {
                    Ok((
                        Output::Normal(NormalOutput {
                            label: label.to_owned(),
                            bounds: bounds.clone(),
                            connectors: vec![],
                        }),
                        bounds.end,
                    ))
                }
            })
            .unwrap_or_else(|| Ok((Output::new_fallback(label), inc)))
    }
}

fn build_outputs(
    config: &HashMap<String, OutputConfig>,
) -> Result<(Vec<NormalOutput>, Vec<FallbackOutput>)> {
    Ok(config
        .iter()
        .try_fold(
            (0.0, vec![]),
            |(inc, mut v), (next_label, next_cfg)| -> Result<(f32, Vec<Output>)> {
                let (output, inc_next) = Output::try_make_next(inc, next_label, next_cfg)?;
                v.push(output);
                Ok((inc_next, v))
            },
        )
        .map(|t| t.1)?
        .into_iter()
        .fold((vec![], vec![]), |(mut v_normal, mut v_fallback), next| {
            match next {
                Output::Normal(output) => v_normal.push(output),
                Output::Fallback(output) => v_fallback.push(output),
            }
            (v_normal, v_fallback)
        }))
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Debug)]
enum ProcessorEvent {
    Build(oneshot::Sender<Result<()>>),
    Start,
    Stop,
    ConnectInput(String, String, Connector, oneshot::Sender<Result<()>>),
    Packet(Packet),
}

pub struct ProcessorState {
    name: String,
    input: SimpleInput,
    controller: Controller,
    config: Config,
    outputs: Vec<NormalOutput>,
    fallbacks: Vec<FallbackOutput>,
}

impl ProcessorState {
    fn launch(
        name: String,
        config: Config,
        controller: Controller,
        outputs: Vec<NormalOutput>,
        fallbacks: Vec<FallbackOutput>,
        rx: UnboundedReceiver<ProcessorEvent>,
    ) -> impl Future<Output = ()> {
        let state = ProcessorState {
            name: name.clone(),
            input: SimpleInput::new(&name, 1024),
            controller,
            config,
            outputs,
            fallbacks,
        };
        processor_tracing_scope!("splitter", name, state.run(rx))
    }

    async fn build(&mut self) -> Result<()> {
        for input in &self.config.inputs {
            tracing::debug!(input, "connect input");
            self.controller
                .connect(&self.name, input, self.input.get_connector())
                .await?;
        }
        Ok(())
    }

    async fn start(&mut self) {}

    async fn connect(
        &mut self,
        destination: String,
        label: String,
        input: Connector,
    ) -> Result<()> {
        if let Some(output) = self.outputs.iter_mut().find(|output| output.label == label) {
            output.connectors.push(input);
            tracing::info!("connected '{}' to destination '{}'", label, destination);
            Ok(())
        } else if let Some(output) = self
            .fallbacks
            .iter_mut()
            .find(|output| output.label == label)
        {
            output.connectors.push(input);
            tracing::info!("connected '{}' to destination '{}'", label, destination);
            Ok(())
        } else {
            Err(anyhow!("no output found for label '{}'", label))
        }
    }

    fn send_packet_fallback(&mut self, packet: Packet) {
        if !self.fallbacks.is_empty() {
            let index = rand::thread_rng().gen_range(0..self.fallbacks.len());
            util::send_packet_to(&mut self.fallbacks[index].connectors, packet)
        }
    }

    fn send_packet(&mut self, packet: Packet) {
        let selector = rand::random::<f32>();
        if let Some(output) = self.outputs.iter_mut().find(|f| f.select(selector)) {
            util::send_packet_to(&mut output.connectors, packet);
        } else {
            self.send_packet_fallback(packet)
        }
    }

    async fn run(mut self, mut rx_ctl: UnboundedReceiver<ProcessorEvent>) {
        tracing::debug!("running");
        loop {
            match select! {
                packet = self.input.receive() => {
                    ProcessorEvent::Packet(packet)
                },
                msg = rx_ctl.recv() => {
                    match msg {
                        Some(msg) => msg,
                        None => ProcessorEvent::Stop
                    }
                }
            } {
                ProcessorEvent::Build(responder) => {
                    responder.send(self.build().await).ok();
                }
                ProcessorEvent::Start => self.start().await,
                ProcessorEvent::ConnectInput(destination, label, connector, responder) => {
                    responder
                        .send(self.connect(destination, label, connector).await)
                        .ok();
                    tracing::trace!("build complete");
                }
                ProcessorEvent::Stop => break,
                ProcessorEvent::Packet(p) => self.send_packet(p),
            }
        }
        tracing::debug!("done");
    }
}
