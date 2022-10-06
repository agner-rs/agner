use std::sync::Arc;

use agner_actors::{Actor, ActorID, Exit, SpawnOpts, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;

use crate::common::{util, InitType, StartChildError, WithAck};

pub async fn do_start_child<B, A, M>(
    system: System,
    sup_id: ActorID,
    behaviour: B,
    args: A,
    init_type: InitType,
) -> Result<ActorID, StartChildError>
where
    B: for<'a> Actor<'a, A, M>,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
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
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
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
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
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

            if let Err(cancel_error) = util::try_exit(
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
