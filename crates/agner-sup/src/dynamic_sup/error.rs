use std::sync::Arc;

use agner_actor::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum SupError {
	#[error("Seed Missing")]
	SeedMissing,
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
	#[error("System Error")]
	SystemError(#[source] Arc<dyn Error>),
}

#[derive(Debug, thiserror::Error)]
pub enum StopError {
	#[error("Child not found")]
	NotFound,
}

impl StartError {
	pub(super) fn system_error<E: Error>(sys_err: E) -> Self {
		Self::SystemError(Arc::new(sys_err))
	}
}
