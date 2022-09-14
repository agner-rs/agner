use std::time::Duration;

const DEFAULT_MAX_ACTORS: usize = 1_024;
const DEFAULT_ACTOR_TERMINATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Configuration for [`System`](crate::system::System)
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// max number of actors in the [`System`](crate::system::System)
    pub max_actors: usize,

    /// max duration given for an actor to gracefully terminate
    pub actor_termination_timeout: Duration,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_actors: DEFAULT_MAX_ACTORS,
            actor_termination_timeout: DEFAULT_ACTOR_TERMINATION_TIMEOUT,
        }
    }
}
