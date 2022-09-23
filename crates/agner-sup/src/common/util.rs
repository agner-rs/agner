use std::time::Duration;

use agner_actors::{ActorID, Exit, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;

#[derive(Debug, thiserror::Error)]
#[error("Exit failed")]
pub struct ExitFailed;

pub async fn try_exit(
    system: System,
    actor_id: ActorID,
    attempts: impl IntoIterator<Item = (Exit, Duration)>,
) -> Result<Exit, ExitFailed> {
    for (exit, timeout) in attempts {
        system.exit(actor_id, exit).await;
        if let Ok(actual_exit) = system.wait(actor_id).timeout(timeout).await {
            return Ok(actual_exit)
        }
    }
    Err(ExitFailed)
}
