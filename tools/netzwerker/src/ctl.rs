use std::{collections::HashMap, result};

use tokio::{
    signal::unix::SignalKind,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::Instrument;

use crate::{
    config::Config,
    engine::Engine,
    processor::{traits::ProcessorClientLifecycle, Connector, ProcessorClient},
    signal::SignalWaiter,
};

use crate::error::SendErrorWithContext;

use anyhow::{Context, Result};

#[derive(Debug)]
pub enum ControlMessage {
    Quit,
    Start(Controller),
    StartComplete,
    Stop,
    ConnectInput(String, String, Connector, oneshot::Sender<Result<()>>),
}

#[derive(Debug, Clone)]
pub struct Controller {
    tx: UnboundedSender<ControlMessage>,
}

impl Controller {
    fn new(tx: UnboundedSender<ControlMessage>) -> Self {
        Self { tx }
    }

    pub fn quit(&self) -> Result<()> {
        tracing::debug!("send quit message to controller task");
        self.tx
            .send(ControlMessage::Quit)
            .send_err_ctx("Quit", "Controller")
    }

    pub fn start(&self) -> Result<()> {
        tracing::debug!("send start message to controller task");
        self.tx
            .send(ControlMessage::Start(self.clone()))
            .send_err_ctx("Start", "Controller")
    }

    pub fn on_started(&self) -> Result<()> {
        self.tx
            .send(ControlMessage::StartComplete)
            .send_err_ctx("StartComplete", "Controller")
    }

    pub fn stop(&self) -> Result<()> {
        tracing::debug!("send stop message to controller task");
        self.tx
            .send(ControlMessage::Stop)
            .send_err_ctx("Stop", "Controller")
    }

    pub async fn connect_input(&self, destination: &str, label: &str, input: Connector) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ControlMessage::ConnectInput(
                destination.to_owned(),
                label.to_owned(),
                input,
                tx,
            ))
            .send_err_ctx("ConnectInput", "Controller")?;
        rx.await.respond_err_ctx("ConnectInput", "Controller")?
    }
}

pub struct ControlProcessorState {
    engine: Engine,
    startup_task: Option<JoinHandle<result::Result<(), Vec<anyhow::Error>>>>,
    signal_waiter: Option<SignalWaiter>,
}

impl ControlProcessorState {
    async fn new(engine: Engine, ctl: Controller) -> Self {
        Self {
            engine,
            startup_task: None,
            signal_waiter: Some(
                SignalWaiter::try_new(vec![SignalKind::interrupt(), SignalKind::terminate()], ctl)
                    .await
                    .expect("failed to register signal handlers"),
            ),
        }
    }

    async fn stop_processors(&mut self) -> Result<()> {
        self.engine
            .stop()
            .await
            .context("failed to stop all processors")
    }

    async fn quit(&mut self) -> Result<()> {
        tracing::debug!("quit message received");
        self.stop_processors().await?;
        if let Some(waiter) = self.signal_waiter.take() {
            waiter.join().await.unwrap()
        }
        Ok(())
    }

    async fn abort(&mut self) -> Result<()> {
        tracing::debug!("abort message received");
        self.stop_processors().await?;
        if let Some(waiter) = self.signal_waiter.take() {
            waiter.cancel().await.unwrap()
        }
        Ok(())
    }

    async fn run_build_impl(
        procs: &HashMap<String, ProcessorClient>,
    ) -> result::Result<(), Vec<anyhow::Error>> {
        let mut errs = vec![];
        for (name, proc_client) in procs {
            if let Err(e) = proc_client.build()
                    .await
                    .with_context(|| format!("failed to build processor '{}'", name)) {
                        errs.push(e);
                    }
        }
        if errs.is_empty() {
            Ok(())
        }
        else {
            Err(errs)
        }
    }

    async fn run_start_impl(
        procs: &HashMap<String, ProcessorClient>,
    ) -> result::Result<(), Vec<anyhow::Error>> {
        let mut errs = vec![];
        for (name, proc_client) in procs {
            if let Err(e) = proc_client.start()
                    .await
                    .with_context(|| format!("failed to start processor '{}'", name)) {
                        errs.push(e);
                    }
        }
        if errs.is_empty() {
            Ok(())
        }
        else {
            Err(errs)
        }
    }

    async fn process_start(
        procs: HashMap<String, ProcessorClient>,
    ) -> result::Result<(), Vec<anyhow::Error>> {
        tracing::debug!("build all processor connections");
        Self::run_build_impl(&procs).await?;
        tracing::debug!("start all processors");
        Self::run_start_impl(&procs).await?;
        Ok(())
    }

    async fn run_start(
        ctl: Controller,
        procs: HashMap<String, ProcessorClient>,
    ) -> result::Result<(), Vec<anyhow::Error>> {
        let r = Self::process_start(procs).await;
        ctl.on_started().unwrap();
        r
    }

    async fn start(&mut self, ctl: Controller) {
        tracing::trace!("launch startup task");
        self.startup_task = Some(tokio::task::spawn(
            Self::run_start(ctl, self.engine.processors())
                .instrument(tracing::span!(tracing::Level::INFO, "startup")),
        ));
        tracing::trace!("startup task launched");
    }

    async fn on_start_complete(&mut self) {
        match self.startup_task.take() {
            Some(task) => match task.await.unwrap() {
                Ok(_) => {
                    tracing::info!("all processors started")
                }
                Err(e) => {
                    tracing::error!(
                        "startup failed, one or more processors were not started successfully:\n{}",
                        e.into_iter()
                            .enumerate()
                            .map(|(i, e)| { format!("{}: {:?}", i, e) })
                            .collect::<String>()
                    );
                    self.abort().await.unwrap();
                }
            },
            None => {
                tracing::warn!("received startup complete message, but no start-job is running")
            }
        }
    }

    async fn stop(&mut self) -> Result<()> {
        self.quit().await
    }

    async fn run(mut self, mut rx: UnboundedReceiver<ControlMessage>, ctl: Controller) {
        tracing::debug!("control task started");
        self.start(ctl).await;
        while let Some(msg) = rx.recv().await {
            match msg {
                ControlMessage::Quit => {
                    self.quit().await.unwrap();
                }
                ControlMessage::Start(ctl) => {
                    self.start(ctl).await;
                }
                ControlMessage::Stop => {
                    self.stop().await.unwrap();
                }
                ControlMessage::ConnectInput(destination, name, input, responder) => {
                    responder
                        .send(self.engine.connect_input(&destination, &name, input).await)
                        .ok();
                }
                ControlMessage::StartComplete => self.on_start_complete().await,
            }
        }
        tracing::debug!("control task done");
    }
}

pub struct ControlProcessor {
    ctl: Controller,
    task: JoinHandle<()>,
}

impl ControlProcessor {
    pub async fn try_new(config: &Config) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let ctl = Controller::new(tx);
        let engine = Engine::try_new(ctl.clone(), config)
            .instrument(tracing::span!(tracing::Level::ERROR, "init"))
            .await?;
        Ok(Self {
            ctl: ctl.clone(),
            task: tokio::task::spawn(
                ControlProcessorState::new(engine, ctl.clone())
                    .await
                    .run(rx, ctl)
                    .instrument(tracing::span!(tracing::Level::ERROR, "ctl")),
            ),
        })
    }

    pub fn controller(&self) -> Controller {
        self.ctl.clone()
    }

    pub async fn join(self) -> result::Result<(), tokio::task::JoinError> {
        let handle = self.task;
        drop(self.ctl);
        handle.await
    }
}
