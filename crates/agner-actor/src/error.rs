use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

use crate::{ActorID, Signal};

const FORMAT_EXIT_ERROR_MAX_DEPTH: usize = 10;

pub trait Error: StdError + Send + Sync + 'static {}

pub type AnyError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub enum ExitReason {
	Normal,
	Error(ExitError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ExitError {
	#[error("Well-known")]
	WellKnown(#[source] Arc<WellKnownReason>),

	#[error("Actor-specific error")]
	Actor(#[source] Arc<dyn Error>),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum WellKnownReason {
	#[error("Kill")]
	Kill,

	#[error("Shutdown")]
	Shutdown,

	#[error("Linked Actor Exited: {}", _0)]
	LinkExit(ActorID),

	#[error("Unhandled Signal")]
	UnhandledSignal(#[source] Arc<Signal>),

	#[error("No Actor: {}", _0)]
	NoActor(ActorID),
}

impl<E> Error for E where E: StdError + Send + Sync + 'static {}

impl From<ExitError> for ExitReason {
	fn from(e: ExitError) -> Self {
		Self::Error(e)
	}
}

impl From<WellKnownReason> for ExitError {
	fn from(inner: WellKnownReason) -> Self {
		Self::well_known(inner)
	}
}

impl From<WellKnownReason> for ExitReason {
	fn from(ge: WellKnownReason) -> Self {
		ExitError::from(ge).into()
	}
}

impl ExitReason {
	pub fn kill() -> Self {
		Self::well_known(WellKnownReason::Kill)
	}
	pub fn shutdown() -> Self {
		Self::well_known(WellKnownReason::Shutdown)
	}
	pub fn well_known(ge: WellKnownReason) -> Self {
		Self::Error(ExitError::well_known(ge))
	}
	pub fn error_actor<E: Error>(e: E) -> Self {
		Self::Error(ExitError::actor(e))
	}
}

impl WellKnownReason {
	pub fn unhandled_signal(sig: Signal) -> Self {
		Self::UnhandledSignal(Arc::new(sig))
	}
	pub fn linked_exit(offender: ActorID) -> Self {
		Self::LinkExit(offender)
	}
}

impl ExitError {
	pub fn well_known(inner: WellKnownReason) -> Self {
		Self::WellKnown(Arc::new(inner))
	}
	pub fn actor<E: Error>(inner: E) -> Self {
		Self::Actor(Arc::new(inner))
	}
}

impl Default for ExitReason {
	fn default() -> Self {
		Self::Normal
	}
}

impl fmt::Display for ExitReason {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Normal => write!(f, "Normal"),
			Self::Error(reason) => format_error_chain(f, reason, FORMAT_EXIT_ERROR_MAX_DEPTH),
		}
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
