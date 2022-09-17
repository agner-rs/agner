use std::time::{Duration, Instant};

use agner_actors::{Actor, ActorID, ExitReason, SpawnOpts, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;
use tokio::sync::oneshot;
use tokio::time::error::Elapsed;

use crate::Registered;
use agner_actors::SysSpawnError;

#[derive(Debug, thiserror::Error)]
pub enum StartChildError {
    #[error("System.spawn error")]
    Spawn(#[source] SysSpawnError),

    #[error("init-ack: oneshot-rx closed")]
    InitAckBrokenPipe,

    #[error("Timeout")]
    Timeout(#[source] Elapsed),

    #[error("oneshot-rx error")]
    Rx(#[source] oneshot::error::RecvError),
}

#[derive(Debug, thiserror::Error)]
pub enum StopChildError {
    #[error("Timeout")]
    Timeout,
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

pub async fn stop_child(
    system: System,
    sup_id: ActorID,
    child_id: ActorID,
    stop_timeout: Duration,
    exit_reason: ExitReason,
) -> Result<(), StopChildError> {
    let t0 = Instant::now();
    log::trace!(
        "[{}] terminating child {} [timeout: {:?}, reason: {}]",
        sup_id,
        child_id,
        stop_timeout,
        exit_reason.pp()
    );
    system.exit(child_id, exit_reason.to_owned()).await;
    let child_exited = system.wait(child_id);
    let child_exited_or_timeout = child_exited.timeout(stop_timeout);

    match child_exited_or_timeout.await {
        Ok(exit_reason) => {
            log::trace!(
                "[{}] child {} exited [elapsed: {:?}, timeout: {:?}, reason: {}]",
                sup_id,
                child_id,
                t0.elapsed(),
                stop_timeout,
                exit_reason.pp()
            );
            Ok(())
        },
        Err(_elapsed) => {
            log::trace!(
                "[{}] child {} timed out on termination. Killing [elapsed: {:?}, timeout: {:?}]",
                sup_id,
                child_id,
                t0.elapsed(),
                stop_timeout
            );
            system.exit(child_id, ExitReason::Kill).await;
            let child_exited = system.wait(child_id);
            let child_exited_or_timeout = child_exited.timeout(stop_timeout);

            match child_exited_or_timeout.await {
                Ok(exit_reason) => {
                    log::trace!(
                        "[{}] child {} killed [elapsed: {:?}, reason: {}]",
                        sup_id,
                        child_id,
                        t0.elapsed(),
                        exit_reason.pp()
                    );
                    Ok(())
                },
                Err(_elapsed) => {
                    log::trace!(
                        "[{}] child {} kill timed out [elapsed: {:?}]",
                        sup_id,
                        child_id,
                        t0.elapsed()
                    );
                    Err(StopChildError::Timeout)
                },
            }
        },
    }
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

    let init_ack_with_timeout = init_ack_rx.timeout(init_timeout);
    let child_id_result = init_ack_with_timeout
        .await
        .map(|opt| opt.ok_or(StartChildError::InitAckBrokenPipe))
        .err_flatten_in();

    if let Err(reason) = child_id_result.as_ref() {
        log::trace!("[{}] init-ack error: {}. Terminating intermediary", sup_id, reason);

        system.exit(intermediary_id, ExitReason::Shutdown(None)).await;
        if system.wait(intermediary_id).timeout(stop_timeout).await.is_err() {
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

impl From<Elapsed> for StartChildError {
    fn from(elapsed: Elapsed) -> Self {
        Self::Timeout(elapsed)
    }
}
impl From<oneshot::error::RecvError> for StartChildError {
    fn from(recv_error: oneshot::error::RecvError) -> Self {
        Self::Rx(recv_error)
    }
}
