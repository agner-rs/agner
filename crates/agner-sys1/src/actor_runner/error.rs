#[derive(Debug, thiserror::Error)]
pub enum ActorRunnerError {
	#[error("rx[{}] closed", _0)]
	RxClosed(&'static str),

	#[error("inbox full")]
	InboxFull,
}
