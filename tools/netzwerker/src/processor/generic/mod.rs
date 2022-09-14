#![allow(unused)]

pub mod connector;

use std::marker::PhantomData;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tracing::{Event, Instrument};

use crate::ctl::Controller;
use crate::util;

use super::{traits, Connector, ProcessorClient};

#[async_trait]
pub trait ProcessorImplementation<Event>: Send + Sync + Sized
where
    Event: Send + Sync,
{
    async fn select(&mut self, ctl: &Controller) -> Option<Event> {
        futures::future::pending().await
    }

    async fn start(&mut self, ctl: &Controller) -> Result<()>;
    async fn stop(&mut self, ctl: &Controller) -> Result<()>;
    async fn build(&mut self, ctl: &Controller) -> Result<()>;

    async fn connect(&mut self, dest: &str, label: &str, input: Connector) -> Result<()> {
        Err(anyhow!("this processor has no outputs to connect to"))
    }

    async fn event(&mut self, e: Event);
}

#[derive(Debug)]
pub enum GenericProcessorEvent {
    Start(oneshot::Sender<Result<()>>),
    Stop(oneshot::Sender<Result<()>>),
    Build(oneshot::Sender<Result<()>>),
    Connect(String, String, Connector, oneshot::Sender<Result<()>>),
    Term,
}

pub struct GenericProcessor {
    pub tx: mpsc::UnboundedSender<GenericProcessorEvent>,
    pub job: tokio::task::JoinHandle<()>,
}

impl GenericProcessor {
    pub async fn try_new<E: 'static, I: ProcessorImplementation<E> + 'static>(
        ctl: Controller,
        implementation: I,
    ) -> Result<Self>
    where
        E: Send + Sync,
    {
        let state = GenericProcessorState::<E, I>::try_new(ctl, implementation)?;
        let (tx, rx) = mpsc::unbounded_channel();
        Ok(Self {
            tx,
            job: tokio::task::spawn(state.run(rx).in_current_span()),
        })
    }
}

#[async_trait]
impl traits::ProcessorJoin for GenericProcessor {
    async fn join(self) -> Result<()> {
        Ok(self.job.await?)
    }
}

impl traits::ProcessorGetClient for GenericProcessor {
    fn client(&self) -> super::ProcessorClient {
        ProcessorClient::GenericProcessorClient(GenericProcessorClient::new(self.tx.clone()))
    }
}

#[derive(Clone)]
pub struct GenericProcessorClient {
    tx: mpsc::UnboundedSender<GenericProcessorEvent>,
}

impl GenericProcessorClient {
    fn new(tx: mpsc::UnboundedSender<GenericProcessorEvent>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl traits::ProcessorClientLifecycle for GenericProcessorClient {
    async fn build(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(GenericProcessorEvent::Build(tx))?;
        rx.await?
    }

    async fn start(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(GenericProcessorEvent::Start(tx))?;
        rx.await?
    }
    async fn stop(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(GenericProcessorEvent::Stop(tx))?;
        rx.await?
    }
}

#[async_trait]
impl traits::ProcessorClientConnectInput for GenericProcessorClient {
    async fn connect(&self, destination: &str, label: &str, input: Connector) -> Result<()> {
        todo!()
    }
}

pub struct GenericProcessorState<E, I: ProcessorImplementation<E>>
where
    E: Send + Sync,
{
    implementation: I,
    ctl: Controller,
    _p: PhantomData<E>,
}

impl<E, I> GenericProcessorState<E, I>
where
    I: ProcessorImplementation<E>,
    E: Send + Sync,
{
    pub fn try_new(ctl: Controller, implementation: I) -> Result<Self> {
        Ok(Self {
            implementation,
            ctl,
            _p: Default::default(),
        })
    }

    pub async fn run(mut self, mut rx: mpsc::UnboundedReceiver<GenericProcessorEvent>) {
        loop {
            if let Some(event) = select! {
                opt_event = rx.recv() => {
                    match opt_event {
                        Some(e) => Some(e),
                        None => Some(GenericProcessorEvent::Term),
                    }
                }
                opt_user_event = self.implementation.select(&self.ctl) => {
                    if let Some(event) = opt_user_event {
                        self.implementation.event(event);
                    }
                    None
                }
            } {
                match event {
                    GenericProcessorEvent::Start(responder) => {
                        responder
                            .send(self.implementation.start(&self.ctl).await)
                            .ok();
                    }
                    GenericProcessorEvent::Stop(responder) => {
                        responder
                            .send(self.implementation.stop(&self.ctl).await)
                            .ok();
                        break;
                    }
                    GenericProcessorEvent::Build(responder) => {
                        responder
                            .send(self.implementation.build(&self.ctl).await)
                            .ok();
                    }
                    GenericProcessorEvent::Connect(dest, label, connector, responder) => {
                        responder
                            .send(self.implementation.connect(&dest, &label, connector).await)
                            .ok();
                    }
                    GenericProcessorEvent::Term => break,
                }
            }
        }
    }
}

#[macro_export]
macro_rules! generic_processor {
    ($ty:literal, $name:expr, $controller:expr, $implementation:expr) => {
        $crate::processor::Processor::GenericProcessor({
            let state = $crate::processor::generic::GenericProcessorState::try_new(
                $controller,
                $implementation,
            )?;
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            $crate::processor::generic::GenericProcessor {
                tx,
                job: tokio::task::spawn($crate::util::processor_tracing_scope!(
                    $ty,
                    $name,
                    state.run(rx)
                )),
            }
        })
    };
}

pub use generic_processor;

fn test() {}
