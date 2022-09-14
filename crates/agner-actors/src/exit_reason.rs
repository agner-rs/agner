use std::error::Error as StdError;
use std::fmt;
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

impl ExitReason {
	pub fn pp<'a>(&'a self) -> impl fmt::Display + 'a {
		ErrorPP(self)
	}
}

const FORMAT_EXIT_ERROR_MAX_DEPTH: usize = 10;

struct ErrorPP<'a>(&'a dyn StdError);

impl<'a> fmt::Display for ErrorPP<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		format_error_chain(f, self.0, FORMAT_EXIT_ERROR_MAX_DEPTH)
	}
}

fn format_error_chain(
	f: &mut fmt::Formatter,
	reason: &dyn StdError,
	hops_left: usize,
) -> fmt::Result {
	if hops_left == 0 {
		write!(f, "...")?;
	} else {
		write!(f, "{}", reason)?;
		if let Some(source) = reason.source() {
			write!(f, " << ")?;
			format_error_chain(f, source, hops_left - 1)?;
		}
	}
	Ok(())
}
