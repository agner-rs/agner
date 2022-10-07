use std::sync::Arc;

use tokio::sync::oneshot;

use agner_actors::system_error::SysSpawnError;
use agner_actors::{Actor, ActorID, Exit, SpawnOpts, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;

use crate::common::{stop_child, InitType, WithAck};

#[derive(Debug, Clone, thiserror::Error)]
pub enum StartChildError {
    #[error("System failed to spawn child")]
    SysSpawnError(#[source] Arc<SysSpawnError>),

    #[error("Init-ack failure")]
    InitAckFailure(#[source] Exit),

    #[error("Timeout")]
    Timeout(#[source] Arc<tokio::time::error::Elapsed>),

    #[error("oneshot-rx failure")]
    OneshotRx(#[source] oneshot::error::RecvError),
}

pub async fn start_child<B, A, M>(
    system: System,
    sup_id: ActorID,
    behaviour: B,
    args: A,
    init_type: InitType,
) -> Result<ActorID, StartChildError>
where
    B: for<'a> Actor<'a, A, M>,
    B: Send + 'static,
    A: Send + 'static,
    M: Send + Unpin + 'static,
{
    log::trace!("[{}|start_child] starting child", sup_id);

    let child_id = match init_type {
        InitType::NoAck => do_start_child_no_ack(&system, sup_id, behaviour, args).await?,
        InitType::WithAck(with_ack) =>
            do_start_child_init_ack(&system, sup_id, behaviour, args, with_ack).await?,
    };

    system.put_data(child_id, crate::common::ParentActor(sup_id)).await;

    Ok(child_id)
}

async fn do_start_child_no_ack<B, A, M>(
    system: &System,
    sup_id: ActorID,
    behaviour: B,
    args: A,
) -> Result<ActorID, StartChildError>
where
    B: for<'a> Actor<'a, A, M>,
    B: Send + 'static,
    A: Send + 'static,
    M: Send + Unpin + 'static,
{
    let spawn_opts = SpawnOpts::new().with_link(sup_id);
    let child_id = system.spawn(behaviour, args, spawn_opts).await?;
    log::trace!("[{}|start_child_no_ack] started [child_id: {}]", sup_id, child_id,);

    Ok(child_id)
}
async fn do_start_child_init_ack<B, A, M>(
    system: &System,
    sup_id: ActorID,
    behaviour: B,
    args: A,
    with_ack: WithAck,
) -> Result<ActorID, StartChildError>
where
    B: for<'a> Actor<'a, A, M>,
    B: Send + 'static,
    A: Send + 'static,
    M: Send + Unpin + 'static,
{
    let (init_ack_tx, init_ack_rx) = agner_init_ack::new_channel();
    let spawn_opts = SpawnOpts::new().with_data(init_ack_tx);
    let intermediary_id = system.spawn(behaviour, args, spawn_opts).await?;

    let init_ack_result = init_ack_rx
        .timeout(with_ack.init_timeout)
        .await
        .map_err(|elapsed| StartChildError::Timeout(Arc::new(elapsed)))
        .map(|id_result| id_result.map_err(StartChildError::InitAckFailure))
        .err_flatten_in();

    match init_ack_result {
        Ok(child_id) => {
            system.link(sup_id, child_id).await;

            log::trace!(
                "[{}|start_child_init_ack] init-ack success [child_id: {}]",
                sup_id,
                child_id,
            );

            Ok(child_id)
        },
        Err(reason) => {
            log::warn!("[{}|start_child_init_ack] canceling init [error: {}]", sup_id, reason.pp());

            if let Err(cancel_error) = stop_child::stop_child(
                system.to_owned(),
                intermediary_id,
                [
                    (
                        Exit::shutdown_with_source(Arc::new(reason.to_owned())),
                        with_ack.stop_timeout,
                    ),
                    (Exit::kill(), with_ack.stop_timeout),
                ],
            )
            .await
            {
                log::error!("[{}|start_child_init_ack] failed to terminate intermediary [intermediary_id: {}, reason: {}]", sup_id, intermediary_id, cancel_error.pp());
            }

            Err(reason)
        },
    }
}

impl From<SysSpawnError> for StartChildError {
    fn from(e: SysSpawnError) -> Self {
        Self::SysSpawnError(Arc::new(e))
    }
}
impl From<tokio::time::error::Elapsed> for StartChildError {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(Arc::new(e))
    }
}
impl From<oneshot::error::RecvError> for StartChildError {
    fn from(e: oneshot::error::RecvError) -> Self {
        Self::OneshotRx(e)
    }
}
