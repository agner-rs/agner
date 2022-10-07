use std::time::Duration;

use agner_actors::{ActorID, Exit, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;

const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_KILL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
#[error("Exit failed")]
pub struct StopChildError;

#[derive(Debug, Clone)]
pub struct ShutdownSequence(Vec<(Exit, Duration)>);

/// Stop the child in accordance with the supervision design principles.
pub async fn stop_child(
    system: System,
    actor_id: ActorID,
    shutdown_sequence: ShutdownSequence,
) -> Result<Exit, StopChildError> {
    for (exit, timeout) in shutdown_sequence.0.into_iter() {
        system.exit(actor_id, exit).await;
        if let Ok(actual_exit) = system.wait(actor_id).timeout(timeout).await {
            return Ok(actual_exit)
        }
    }
    Err(StopChildError)
}

impl Default for ShutdownSequence {
    fn default() -> Self {
        [(Exit::shutdown(), DEFAULT_SHUTDOWN_TIMEOUT), (Exit::kill(), DEFAULT_KILL_TIMEOUT)].into()
    }
}

impl ShutdownSequence {
    pub fn empty() -> Self {
        Self(vec![])
    }
    pub fn add(mut self, exit: Exit, timeout: Duration) -> Self {
        self.0.push((exit, timeout));
        self
    }
}

impl<I> From<I> for ShutdownSequence
where
    I: IntoIterator<Item = (Exit, Duration)>,
{
    fn from(seq: I) -> Self {
        Self(seq.into_iter().collect())
    }
}
