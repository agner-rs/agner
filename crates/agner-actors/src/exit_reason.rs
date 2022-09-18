use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::imports::ArcError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExitReason {
    #[error("Well known")]
    WellKnown(#[source] WellKnown),

    #[error("Actor backend failure")]
    Backend(#[source] BackendFailure),

    #[error("Generic")]
    Generic(#[source] ArcError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum WellKnown {
    #[error("Normal")]
    Normal,

    #[error("Kill")]
    Kill,

    #[error("Exited: {}", _0)]
    Exited(ActorID, #[source] Box<ExitReason>),

    #[error("No Actor")]
    NoActor,

    #[error("Shutdown")]
    Shutdown(#[source] Option<ArcError>),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BackendFailure {
    #[error("Inbox Full: {}", _0)]
    InboxFull(&'static str),

    #[error("Rx Closed: {}", _0)]
    RxClosed(&'static str),
}

impl Default for ExitReason {
    fn default() -> Self {
        Self::WellKnown(WellKnown::Normal)
    }
}
impl From<WellKnown> for ExitReason {
    fn from(e: WellKnown) -> Self {
        Self::WellKnown(e)
    }
}
impl From<BackendFailure> for ExitReason {
    fn from(e: BackendFailure) -> Self {
        Self::Backend(e)
    }
}

impl ExitReason {
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::WellKnown(WellKnown::Normal))
    }
    pub fn is_kill(&self) -> bool {
        matches!(self, Self::WellKnown(WellKnown::Kill))
    }
    pub fn normal() -> Self {
        WellKnown::Normal.into()
    }
    pub fn kill() -> Self {
        WellKnown::Kill.into()
    }
    pub fn exited(who: ActorID, reason: impl Into<Box<Self>>) -> Self {
        WellKnown::Exited(who, reason.into()).into()
    }
    pub fn no_actor() -> Self {
        WellKnown::NoActor.into()
    }
    pub fn shutdown() -> Self {
        WellKnown::Shutdown(None).into()
    }
    pub fn shutdown_with_source(source: ArcError) -> Self {
        WellKnown::Shutdown(Some(source)).into()
    }

    pub fn generic<E: std::error::Error + Send + Sync + 'static>(e: E) -> ExitReason {
        Self::Generic(Arc::new(e))
    }
}
