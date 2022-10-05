use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::imports::ArcError;

mod into_exit;

/// An reason an actor exited.
///
/// Exit reasons are supposed to be cheaply cloneable, as when an actor fails each linked actor
/// receives a signal containing a clone of that reason.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Exit {
    #[error("Well known")]
    Standard(#[source] WellKnown),

    #[error("Actor backend failure")]
    Backend(#[source] BackendFailure),

    #[error("Custom")]
    Custom(#[source] ArcError),
}

/// Standard exit reasons.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WellKnown {
    #[error("Normal")]
    Normal,

    #[error("Kill")]
    Kill,

    #[error("Exited: {}", _0)]
    Linked(ActorID, #[source] Box<Exit>),

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
        Self::Standard(WellKnown::Normal)
    }
}
impl From<WellKnown> for Exit {
    fn from(e: WellKnown) -> Self {
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
        matches!(self, Self::Standard(WellKnown::Normal))
    }
    pub fn is_kill(&self) -> bool {
        matches!(self, Self::Standard(WellKnown::Kill))
    }
    pub fn is_linked(&self) -> bool {
        matches!(self, Self::Standard(WellKnown::Linked(_, _)))
    }
    pub fn is_no_actor(&self) -> bool {
        matches!(self, Self::Standard(WellKnown::NoActor))
    }
    pub fn is_shutdown(&self) -> bool {
        matches!(self, Self::Standard(WellKnown::Shutdown(_)))
    }
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    pub fn normal() -> Self {
        WellKnown::Normal.into()
    }
    pub fn kill() -> Self {
        WellKnown::Kill.into()
    }
    pub fn linked(who: ActorID, reason: impl Into<Box<Self>>) -> Self {
        WellKnown::Linked(who, reason.into()).into()
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

    pub fn custom<E: std::error::Error + Send + Sync + 'static>(e: E) -> Exit {
        Self::Custom(Arc::new(e))
    }
}
