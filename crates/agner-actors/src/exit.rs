use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::imports::ArcError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Exit {
    #[error("Well known")]
    Standard(#[source] ExitStandard),

    #[error("Actor backend failure")]
    Backend(#[source] BackendFailure),

    #[error("Custom")]
    Custom(#[source] ArcError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExitStandard {
    #[error("Normal")]
    Normal,

    #[error("Kill")]
    Kill,

    #[error("Exited: {}", _0)]
    Exited(ActorID, #[source] Box<Exit>),

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

impl Default for Exit {
    fn default() -> Self {
        Self::Standard(ExitStandard::Normal)
    }
}
impl From<ExitStandard> for Exit {
    fn from(e: ExitStandard) -> Self {
        Self::Standard(e)
    }
}
impl From<BackendFailure> for Exit {
    fn from(e: BackendFailure) -> Self {
        Self::Backend(e)
    }
}

impl Exit {
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Standard(ExitStandard::Normal))
    }
    pub fn is_kill(&self) -> bool {
        matches!(self, Self::Standard(ExitStandard::Kill))
    }
    pub fn is_exited(&self) -> bool {
        matches!(self, Self::Standard(ExitStandard::Exited(_, _)))
    }
    pub fn is_no_actor(&self) -> bool {
        matches!(self, Self::Standard(ExitStandard::NoActor))
    }
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::Standard(ExitStandard::Shutdown(_)))
    }

    pub fn normal() -> Self {
        ExitStandard::Normal.into()
    }
    pub fn kill() -> Self {
        ExitStandard::Kill.into()
    }
    pub fn exited(who: ActorID, reason: impl Into<Box<Self>>) -> Self {
        ExitStandard::Exited(who, reason.into()).into()
    }
    pub fn no_actor() -> Self {
        ExitStandard::NoActor.into()
    }
    pub fn shutdown() -> Self {
        ExitStandard::Shutdown(None).into()
    }
    pub fn shutdown_with_source(source: ArcError) -> Self {
        ExitStandard::Shutdown(Some(source)).into()
    }

    pub fn custom<E: std::error::Error + Send + Sync + 'static>(e: E) -> Exit {
        Self::Custom(Arc::new(e))
    }
}
