pub mod config;
use super::{generic::connector::simple::SimpleInput, traits};
use crate::{ctl::Controller, error::SendErrorWithContext, packet::Packet, util};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
pub use config::*;
use tokio::{
    net::UdpSocket,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::Instrument;

/* ------------------------------------------------------------------------------------------------------------ */

pub struct TxProcessor {
    tx: UnboundedSender<ProcessorEvent>,
    job: JoinHandle<()>,
}

impl TxProcessor {
    pub async fn try_new(name: String, config: &Config, ctl: Controller) -> Result<TxProcessor> {
        let sock = config
            .socket()
            .await
            .context("failed to create sending socket from configuration")?;
        let (tx, rx) = unbounded_channel();
        let mut state = TxProcessorState::new(
            name.clone(),
            config,
            SimpleInput::new(&name, 1024),
            sock,
            ctl,
        );
        Ok(Self {
            tx,
            job: tokio::task::spawn(util::processor_tracing_scope!(
                "tx",
                name,
                state.run(rx)
            )),
        })
    }
}

impl traits::ProcessorGetClient for TxProcessor {
    fn client(&self) -> super::ProcessorClient {
        TxProcessorClient::new_wrapped(self.tx.clone())
    }
}

#[async_trait]
impl traits::ProcessorJoin for TxProcessor {
    async fn join(self) -> anyhow::Result<()> {
        Ok(self.job.await?)
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Clone)]
pub struct TxProcessorClient {
    tx: UnboundedSender<ProcessorEvent>,
}

impl TxProcessorClient {
    fn new(tx: UnboundedSender<ProcessorEvent>) -> Self {
        Self { tx }
    }

    fn new_wrapped(tx: UnboundedSender<ProcessorEvent>) -> super::ProcessorClient {
        super::ProcessorClient::TxProcessorClient(Self::new(tx))
    }
}

#[async_trait]
impl traits::ProcessorClientLifecycle for TxProcessorClient {
    async fn build(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ProcessorEvent::Build(tx))
            .send_err_ctx("Build", "TxProcessor")?;
        rx.await?
    }

    async fn start(&self) -> anyhow::Result<()> {
        self.tx
            .send(ProcessorEvent::Start)
            .send_err_ctx("Start", "TxProcessor")
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.tx
            .send(ProcessorEvent::Stop)
            .send_err_ctx("Stop", "Processor")
    }
}

#[async_trait]
impl traits::ProcessorClientConnectInput for TxProcessorClient {
    async fn connect(
        &self,
        _destination: &str,
        _name: &str,
        _input: super::Connector,
    ) -> anyhow::Result<()> {
        Err(anyhow!("cannot connect to a tx processor"))
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Debug)]
enum ProcessorEvent {
    Build(oneshot::Sender<Result<()>>),
    Start,
    Stop,
    Packet(Packet),
}

/* ------------------------------------------------------------------------------------------------------------ */

struct TxProcessorState {
    name: String,
    config: Config,
    input: SimpleInput,
    sock: UdpSocket,
    ctl: Controller,
}

impl TxProcessorState {
    fn new(
        name: String,
        config: &Config,
        input: SimpleInput,
        sock: UdpSocket,
        ctl: Controller,
    ) -> Self {
        Self {
            name,
            config: config.clone(),
            input,
            sock,
            ctl,
        }
    }

    async fn send_packet(&mut self, p: Packet) {
        if let Err(_err) = self.sock.send(&p.data).await {
            // println!("err: {}", err);
        }
    }

    async fn build(&self) -> Result<()> {
        for input in self.config.inputs.iter() {
            tracing::debug!(input, "connect input");
            self.ctl
                .connect(&self.name, input, self.input.get_connector())
                .await?
        }
        Ok(())
    }

    async fn run(&mut self, mut rx: UnboundedReceiver<ProcessorEvent>) {
        tracing::debug!("running");
        loop {
            match tokio::select! {
                packet = self.input.receive() => {
                    ProcessorEvent::Packet(packet)
                }
                event = rx.recv() => {
                    match event {
                        Some(e) => e,
                        None => ProcessorEvent::Stop
                    }
                }
            } {
                ProcessorEvent::Build(responder) => {
                    responder.send(self.build().await).ok();
                }
                ProcessorEvent::Start => {}
                ProcessorEvent::Stop => break,
                ProcessorEvent::Packet(p) => {
                    self.send_packet(p).await;
                }
            }
        }
        tracing::debug!("done");
    }
}
