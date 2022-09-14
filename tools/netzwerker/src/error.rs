use std::fmt::{Debug, Display};

use anyhow::{Context};

use tokio::sync::{mpsc, oneshot};

pub struct SendErrContext(&'static str, &'static str);
pub struct RespondErrContext(&'static str, &'static str);

impl Display for SendErrContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to send message {} to {}: channel was closed",
            self.0, self.1
        )
    }
}

impl Display for RespondErrContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "got no response to {} from {}: channel was closed",
            self.1, self.0
        )
    }
}

pub trait SendErrorWithContext<T, E>: Sized + Context<T, E>
where
    E: std::error::Error + Send + Sync,
{
    fn send_err_ctx(self, msg: &'static str, to: &'static str) -> anyhow::Result<T> {
        self.context(SendErrContext(msg, to))
    }

    fn respond_err_ctx(self, msg: &'static str, to: &'static str) -> anyhow::Result<T> {
        self.context(RespondErrContext(msg, to))
    }
}

impl<T, M> SendErrorWithContext<T, mpsc::error::SendError<M>>
    for Result<T, mpsc::error::SendError<M>>
where
    M: Debug + Send + Sync + 'static,
{
}

impl<T> SendErrorWithContext<T, oneshot::error::RecvError>
    for Result<T, oneshot::error::RecvError>
{
}
