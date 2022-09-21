use std::time::Duration;

use agner_actors::{ActorID, Exit, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;

pub async fn try_exit(
    system: System,
    actor_id: ActorID,
    attempts: impl IntoIterator<Item = (Exit, Duration)>,
) -> Result<Exit, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut attempts = attempts.into_iter();

    while let Some((exit, timeout)) = attempts.next() {
        system.exit(actor_id, exit).await;
        if let Ok(actual_exit) = system.wait(actor_id).timeout(timeout).await {
            return Ok(actual_exit)
        }
    }
    Err("all attempts done".into())
}
