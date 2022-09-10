use std::time::Duration;

const DEFAULT_MAX_ACTORS: usize = 1_024;
const DEFAULT_ACTOR_TERMINATION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct SystemOneConfig {
	pub max_actors: usize,
	pub actor_termination_timeout: Duration,
}

impl Default for SystemOneConfig {
	fn default() -> Self {
		Self {
			max_actors: DEFAULT_MAX_ACTORS,
			actor_termination_timeout: DEFAULT_ACTOR_TERMINATION_TIMEOUT,
		}
	}
}
