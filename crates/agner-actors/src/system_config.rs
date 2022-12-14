use std::sync::Arc;
use std::time::Duration;

use crate::exit_handler::{ExitHandler, NoopExitHandler};

/// Configuration for [`System`](crate::system::System)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SystemConfig {
    /// max number of actors in the [`System`](crate::system::System)
    pub max_actors: usize,

    /// max duration given for an actor to gracefully terminate
    pub actor_termination_timeout: Duration,

    /// exit handler
    #[cfg_attr(feature = "serde", serde(skip, default = "defaults::default_exit_handler"))]
    pub exit_handler: Arc<dyn ExitHandler>,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_actors: defaults::DEFAULT_MAX_ACTORS,
            actor_termination_timeout: defaults::DEFAULT_ACTOR_TERMINATION_TIMEOUT,
            exit_handler: defaults::default_exit_handler(),
        }
    }
}

mod defaults {
    use super::*;

    pub(super) const DEFAULT_MAX_ACTORS: usize = 1_024;
    pub(super) const DEFAULT_ACTOR_TERMINATION_TIMEOUT: Duration = Duration::from_secs(30);

    pub(super) fn default_exit_handler() -> Arc<dyn ExitHandler> {
        Arc::new(NoopExitHandler)
    }
}
