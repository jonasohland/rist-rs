mod config;

use config::*;

use super::{generic::connector::ConnectorCollection, traits, Connector};
use crate::{ctl::Controller, packet::Packet, util};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
pub use config::Config;
use rist_rs_core::static_vec::StaticVec;
use std::{
    net::{IpAddr, SocketAddr},
    result,
};
use tokio::{
    net::UdpSocket,
    select,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};

use tracing::Instrument;

/* ------------------------------------------------------------------------------------------------------------ */

pub struct RxProcessor {
    job: JoinHandle<()>,
    tx: UnboundedSender<ProcessorEvent>,
}

#[async_trait]
impl traits::ProcessorJoin for RxProcessor {
    async fn join(mut self) -> Result<()> {
        Ok(self.job.await?)
    }
}

impl traits::ProcessorGetClient for RxProcessor {
    fn client(&self) -> super::ProcessorClient {
        super::ProcessorClient::RxProcessorClient(RxProcessorClient {
            tx: self.tx.clone(),
        })
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Clone)]
pub struct RxProcessorClient {
    tx: UnboundedSender<ProcessorEvent>,
}

#[async_trait]
impl traits::ProcessorClientLifecycle for RxProcessorClient {
    async fn stop(&self) -> super::Result<()> {
        Ok(self.tx.send(ProcessorEvent::Stop)?)
    }

    async fn start(&self) -> super::Result<()> {
        Ok(self.tx.send(ProcessorEvent::Start)?)
    }

    async fn build(&self) -> super::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(ProcessorEvent::Build(tx))?;
        rx.await?
    }
}

#[async_trait]
impl traits::ProcessorClientConnectInput for RxProcessorClient {
    async fn connect(
        &self,
        destination: &str,
        label: &str,
        connector: Connector,
    ) -> result::Result<(), super::Error> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(ProcessorEvent::ConnectInput(
            destination.to_owned(),
            label.to_owned(),
            connector,
            tx,
        ))?;
        rx.await?
    }
}

/* ------------------------------------------------------------------------------------------------------------ */

#[derive(Debug)]
enum ProcessorEvent {
    Build(oneshot::Sender<Result<()>>),
    Start,
    Stop,
    Process(Vec<u8>, SocketAddr),
    ConnectInput(String, String, Connector, oneshot::Sender<Result<()>>),
}

struct ProcessorState {
    socket: UdpSocket,
    rx: UnboundedReceiver<ProcessorEvent>,
    connectors: ConnectorCollection,
    running: bool,
}

enum RunState {
    Continue,
    Break,
}

impl ProcessorState {
    fn new(socket: UdpSocket, rx: UnboundedReceiver<ProcessorEvent>, _ctl: Controller) -> Self {
        Self {
            socket,
            rx,
            running: false,
            connectors: ConnectorCollection::new(),
        }
    }

    async fn build(&mut self) -> Result<()> {
        Ok(())
    }

    async fn cmd(&mut self, cmd: ProcessorEvent) -> RunState {
        match cmd {
            ProcessorEvent::Stop => RunState::Break,
            ProcessorEvent::Start => {
                self.running = true;
                RunState::Continue
            }
            ProcessorEvent::Build(responder) => {
                responder.send(self.build().await).ok();
                RunState::Continue
            }
            ProcessorEvent::Process(p, a) => {
                self.connectors.send(Packet::new(a, p)).await;
                RunState::Continue
            }
            ProcessorEvent::ConnectInput(destination, label, connector, responder) => {
                responder
                    .send(self.connectors.connect(&destination, &label, connector))
                    .ok();
                RunState::Continue
            }
        }
    }

    async fn receive(socket: &UdpSocket, buf: &mut [u8]) -> Option<ProcessorEvent> {
        match socket.recv_from(buf).await {
            Ok((s, addr)) => Some(ProcessorEvent::Process(Vec::from(buf.split_at(s).0), addr)),
            Err(_) => None,
        }
    }

    async fn run(mut self) {
        let mut stack_buf = StaticVec::new(1600);
        tracing::debug!("running");
        loop {
            if let Some(cmd) = match self.running {
                true => select! {
                    cmd_opt = self.rx.recv() => {
                        match cmd_opt {
                            Some(cmd) => Some(cmd),
                            None => Some(ProcessorEvent::Stop)
                        }
                    }
                    cmd_opt = Self::receive(&self.socket, &mut stack_buf) => {
                        cmd_opt
                    }
                },
                _ => Some(self.rx.recv().await.unwrap_or(ProcessorEvent::Stop)),
            } {
                match self.cmd(cmd).await {
                    RunState::Continue => continue,
                    RunState::Break => break,
                }
            }
        }
        tracing::debug!("done")
    }
}

impl RxProcessor {
    /// Make a bound socket
    async fn make_sock(bind: &BindConfig) -> Result<UdpSocket> {
        let addr = SocketAddr::new(
            match bind.addr {
                None => "::".parse().unwrap(),
                Some(addr) => addr,
            },
            bind.port,
        );
        tracing::info!(address = %addr, "binding socket");
        UdpSocket::bind(addr)
            .await
            .context(format!("failed to bind udp socket to: {}", addr))
    }

    /// Join the provided multicast groups on the socket and return it
    fn join(socket: UdpSocket, join_config: &JoinConfig) -> Result<UdpSocket> {
        if !join_config.group.is_multicast() {
            Err(anyhow::anyhow!(
                "{}, is not a multicast address",
                join_config.group
            ))
        } else {
            match join_config.group {
                IpAddr::V4(group) => {
                    for interface in &join_config.interfaces {
                        tracing::info!(%group, %interface, "join multicast");
                        match interface {
                            IpAddr::V4(ip) => socket
                                .join_multicast_v4(group, *ip)
                                .context(group)
                                .context(*ip)?,
                            _ => Err(anyhow::anyhow!("cannot join v4 group on v6 interface"))?,
                        }
                    }
                    Ok(socket)
                }
                IpAddr::V6(_) => Err(anyhow!("join ipv6 groups not supported yet")),
            }
        }
    }

    /// Spawn a running processor
    pub async fn try_new(name: String, config: &Config, controller: Controller) -> Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let state = ProcessorState::new(
            match &config.join {
                Some(cfg) => Self::join(Self::make_sock(&config.bind).await?, cfg)?,
                None => Self::make_sock(&config.bind).await?,
            },
            rx,
            controller,
        );
        Ok(RxProcessor {
            job: tokio::task::spawn(util::processor_tracing_scope!("rx", name, state.run())),
            tx,
        })
    }
}
