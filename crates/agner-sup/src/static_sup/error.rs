use std::sync::Arc;

use agner_actor::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum SupError {
	#[error("Start Error")]
	StartError(StartError),

	#[error("Stop Error")]
	StopError(StopError),
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
	#[error("Child is already started")]
	AlreadyStarted,

	#[error("No Such Child: {}", _0)]
	NotFound(usize),

	#[error("System Error")]
	SystemError(#[source] Arc<dyn Error>),
}

#[derive(Debug, thiserror::Error)]
pub enum StopError {
	#[error("Child is alrady stopped")]
	AlreadyStopped,

	#[error("Child not found: {}", _0)]
	NotFound(usize),
}

impl StartError {
	pub(super) fn system_error<E: Error>(sys_err: E) -> Self {
		Self::SystemError(Arc::new(sys_err))
	}
}

impl From<StartError> for SupError {
	fn from(e: StartError) -> Self {
		Self::StartError(e)
	}
}

impl From<StopError> for SupError {
	fn from(e: StopError) -> Self {
		Self::StopError(e)
	}
}
