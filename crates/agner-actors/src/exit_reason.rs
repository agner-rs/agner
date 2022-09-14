use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::imports::ArcError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExitReason {
	#[error("Normal")]
	Normal,

	#[error("Exited: {}", _0)]
	Exited(ActorID, #[source] Arc<Self>),

	#[error("No Process")]
	NoProcess,

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
