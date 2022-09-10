#[derive(Debug, thiserror::Error)]
pub enum SystemOneError {
	#[error("Cannot spawn another actor: limit reached")]
	ActorsCountLimit,
}
