use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::imports::ArcError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExitReason {
    #[error("Normal")]
    Normal,

    #[error("Kill")]
    Kill,

    #[error("Exited: {}", _0)]
    Exited(ActorID, #[source] Box<Self>),

    #[error("No Actor")]
    NoActor,

    #[error("Shutdown")]
    Shutdown(#[source] Option<ArcError>),

    #[error("Inbox Full: {}", _0)]
    InboxFull(&'static str),

    #[error("Rx Closed: {}", _0)]
    RxClosed(&'static str),

    #[error("Generic")]
    Generic(#[source] ArcError),
}

impl Default for ExitReason {
    fn default() -> Self {
        Self::Normal
    }
}

impl ExitReason {
    pub fn generic<E: std::error::Error + Send + Sync + 'static>(e: E) -> ExitReason {
        Self::Generic(Arc::new(e))
    }
}
