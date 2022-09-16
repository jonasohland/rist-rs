use futures::Future;
use tokio::{
    io, select,
    signal::unix::{Signal, SignalKind},
    sync::oneshot::{channel, Receiver, Sender},
};
use tracing::Instrument;

use crate::ctl::Controller;

#[derive(Debug)]
pub enum Error {
    Signal(io::Error),
}

type Result<T> = std::result::Result<T, Error>;

struct SignalWrapper {
    signal: Signal,
}

impl SignalWrapper {
    fn new(signal: Signal) -> Self {
        Self { signal }
    }
}

impl Future for SignalWrapper {
    type Output = Option<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.signal.poll_recv(cx)
    }
}

macro_rules! match_sig {
    ($var:expr => {$($sig:ident),*}) => {
        match $var {
            $(libc::$sig => stringify!($sig),)*
            _ => "UNKNOWN"
        }
    };
}

fn signal_name(signal: SignalKind) -> &'static str {
    match_sig! {
        signal.as_raw_value() => {
            SIGHUP,
            SIGINT,
            SIGQUIT,
            SIGILL,
            SIGABRT,
            SIGFPE,
            SIGKILL,
            SIGSEGV,
            SIGPIPE,
            SIGALRM,
            SIGTERM
        }
    }
}

async fn run_signal_waiter(
    signals: Vec<(Signal, SignalKind)>,
    controller: Controller,
    rx: Receiver<()>,
) {
    tokio::pin!(rx);
    let kinds = signals
        .iter()
        .map(|(_, kind)| kind)
        .copied()
        .collect::<Vec<_>>();
    select! {
        r = futures::future::select_all(
            signals
                .into_iter()
                .map(|(s, _)| Box::pin(SignalWrapper::new(s))),
        ) => {
            let (result, index, _) = r;
            if result.is_some() {
                tracing::info!("received {}", signal_name(kinds[index]));
                controller.quit().ok();
            }
        }
        _ = rx => {
            tracing::info!("received shutdown message before a signal arrived");
        }
    };
}

pub struct SignalWaiter {
    tx: Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl SignalWaiter {
    pub async fn try_new(signals: Vec<SignalKind>, ctl: Controller) -> Result<Self> {
        let (tx, rx) = channel();
        Ok(Self {
            tx,
            task: tokio::task::spawn(
                run_signal_waiter(
                    signals
                        .into_iter()
                        .map(|kind| {
                            tokio::signal::unix::signal(kind)
                                .map(|signal| (signal, kind))
                                .map_err(Error::Signal)
                        })
                        .collect::<Result<Vec<(Signal, SignalKind)>>>()?,
                    ctl,
                    rx,
                )
                .instrument(tracing::span!(tracing::Level::DEBUG, "signals")),
            ),
        })
    }

    pub async fn cancel(self) -> std::result::Result<(), tokio::task::JoinError> {
        self.tx.send(()).ok();
        self.task.await
    }

    pub async fn join(self) -> std::result::Result<(), tokio::task::JoinError> {
        drop(self.tx);
        self.task.await
    }
}
