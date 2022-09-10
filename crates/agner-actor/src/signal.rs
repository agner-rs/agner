use crate::error::ExitError;
use crate::ActorID;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Signal {
	#[error("Exit {}; reason: {}", _0, _1)]
	Exit(ActorID, #[source] ExitError),
}
