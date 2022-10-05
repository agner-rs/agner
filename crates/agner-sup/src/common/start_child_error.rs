use std::sync::Arc;
use tokio::sync::oneshot;

use agner_actors::system_error::SysSpawnError;
use agner_actors::Exit;

#[derive(Debug, Clone, thiserror::Error)]
pub enum StartChildError {
    #[error("System failed to spawn child")]
    SysSpawnError(#[source] Arc<SysSpawnError>),

    #[error("Init-ack failure")]
    InitAckFailure(#[source] Exit),

    #[error("Timeout")]
    Timeout(#[source] Arc<tokio::time::error::Elapsed>),

    #[error("oneshot-rx failure")]
    OneshotRx(#[source] oneshot::error::RecvError),
}

impl From<SysSpawnError> for StartChildError {
    fn from(e: SysSpawnError) -> Self {
        Self::SysSpawnError(Arc::new(e))
    }
}
impl From<tokio::time::error::Elapsed> for StartChildError {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(Arc::new(e))
    }
}
impl From<oneshot::error::RecvError> for StartChildError {
    fn from(e: oneshot::error::RecvError) -> Self {
        Self::OneshotRx(e)
    }
}
