use std::time::Duration;

use agner_actors::{Actor, ActorID, ExitReason, SpawnOpts, System};

use crate::dynamic::SpawnError;
use crate::Registered;

#[derive(Debug, thiserror::Error)]
pub enum StartChildError {
    #[error("System.spawn error")]
    Spawn(#[source] SpawnError),

    #[error("init-ack: oneshot-rx closed")]
    InitAckBrokenPipe,

    #[error("init-ack: timeout")]
    InitAckTimeout,
}

pub async fn start_child<ChB, ChA, ChM>(
    system: System,
    sup_id: ActorID,
    child_behaviour: ChB,
    child_arg: ChA,
    init_timeouts: Option<(Duration, Duration)>,
    regs: impl IntoIterator<Item = Registered>,
) -> Result<ActorID, StartChildError>
where
    ChB: for<'a> Actor<'a, ChA, ChM>,
    ChA: Send + Sync + 'static,
    ChM: Send + Sync + Unpin + 'static,
{
    let child_id_result = if let Some((init_timeout, stop_timeout)) = init_timeouts {
        start_child_with_init_ack(
            system,
            sup_id,
            child_behaviour,
            child_arg,
            init_timeout,
            stop_timeout,
        )
        .await
    } else {
        start_child_without_init_ack(system, sup_id, child_behaviour, child_arg).await
    };
    let child_id = child_id_result?;

    for reg in regs {
        reg.update(child_id);
    }

    Ok(child_id)
}

async fn start_child_without_init_ack<ChB, ChA, ChM>(
    system: System,
    sup_id: ActorID,
    child_behaviour: ChB,
    child_arg: ChA,
) -> Result<ActorID, StartChildError>
where
    ChB: for<'a> Actor<'a, ChA, ChM>,
    ChA: Send + Sync + 'static,
    ChM: Send + Sync + Unpin + 'static,
{
    log::trace!(
        "[{}] starting child without init-ack [behaviour: {}; arg: {}; message: {}]",
        sup_id,
        std::any::type_name::<ChB>(),
        std::any::type_name::<ChA>(),
        std::any::type_name::<ChM>()
    );
    let spawn_opts = SpawnOpts::new().with_link(sup_id);
    let child_id = system
        .spawn(child_behaviour, child_arg, spawn_opts)
        .await
        .map_err(StartChildError::Spawn)?;

    log::trace!("[{}] child-id: {}", sup_id, child_id);

    Ok(child_id)
}

async fn start_child_with_init_ack<ChB, ChA, ChM>(
    system: System,
    sup_id: ActorID,
    child_behaviour: ChB,
    child_arg: ChA,
    init_timeout: Duration,
    stop_timeout: Duration,
) -> Result<ActorID, StartChildError>
where
    ChB: for<'a> Actor<'a, ChA, ChM>,
    ChA: Send + Sync + 'static,
    ChM: Send + Sync + Unpin + 'static,
{
    log::trace!(
        "[{}] starting child with init-ack [behaviour: {}; arg: {}; message: {}]",
        sup_id,
        std::any::type_name::<ChB>(),
        std::any::type_name::<ChA>(),
        std::any::type_name::<ChM>()
    );

    let (init_ack_tx, init_ack_rx) = agner_actors::new_init_ack();
    let spawn_opts = SpawnOpts::new().with_init_ack(init_ack_tx);
    let intermediary_id = system
        .spawn(child_behaviour, child_arg, spawn_opts)
        .await
        .map_err(StartChildError::Spawn)?;
    log::trace!("[{}] intermediary-id: {}", sup_id, intermediary_id);

    let init_ack_with_timeout = tokio::time::timeout(init_timeout, init_ack_rx);
    let child_id_result = match init_ack_with_timeout.await {
        Err(_elapsed) => Err(StartChildError::InitAckTimeout),
        Ok(None) => Err(StartChildError::InitAckBrokenPipe),
        Ok(Some(child_id)) => Ok(child_id),
    };

    if let Err(reason) = child_id_result.as_ref() {
        log::trace!("[{}] init-ack error: {}. Terminating intermediary", sup_id, reason);

        system.exit(intermediary_id, ExitReason::Shutdown(None)).await;
        if tokio::time::timeout(stop_timeout, system.wait(intermediary_id)).await.is_err() {
            log::trace!(
                "[{}] intermediary {} took too long to terminate. Killing it.",
                sup_id,
                intermediary_id
            );
            system.exit(intermediary_id, ExitReason::Kill).await;
        }
    }

    let child_id = child_id_result?;
    log::trace!("[{}] child-id: {}", sup_id, child_id);

    system.link(sup_id, child_id).await;

    Ok(child_id)
}