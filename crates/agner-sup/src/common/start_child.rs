use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::process::Output;
use std::time::Duration;

use agner_actors::{Actor, ActorID, Exit, SpawnOpts, SysSpawnError, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;

use crate::common::util;
use crate::service::Service;

#[cfg(test)]
mod tests;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

#[derive(Debug, thiserror::Error)]
pub enum StartChildError {
    #[error("System failed to spawn child")]
    SysSpawnError(#[source] SysSpawnError),

    #[error("Init-ack failure")]
    InitAckFailure,

    #[error("Timeout")]
    Timeout(tokio::time::error::Elapsed),
}

pub trait StartChild<A>: fmt::Debug + Send + Sync + 'static {
    fn start_child(
        self: Box<Self>,
        system: System,
        input_arg: A,
    ) -> BoxedFuture<Result<ActorID, StartChildError>>;
}

#[derive(Debug, Clone, Copy)]
pub enum InitType {
    NoAck,
    WithAck { init_timeout: Duration, stop_timeout: Duration },
}

pub fn without_ack<B, A, M, PS>(
    sup_id: ActorID,
    actor_behaviour: B,
    actor_args: A,
    init_type: InitType,
    provided_services: PS,
) -> Box<dyn StartChild<()>>
where
    PS: IntoIterator<Item = Service>,
    B: for<'a> Actor<'a, A, M>,
    PS: Send + Sync + 'static,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
{
    let start_child = StartChildImpl {
        sup_id,
        actor_behaviour,
        actor_args,
        actor_message: PhantomData::<M>,
        provided_services,
        init_type,
    };
    Box::new(start_child)
}

struct StartChildImpl<B, A, M, PS> {
    sup_id: ActorID,
    provided_services: PS,
    actor_behaviour: B,
    actor_args: A,
    actor_message: PhantomData<M>,
    init_type: InitType,
}

impl<B, A, M, PS> StartChild<()> for StartChildImpl<B, A, M, PS>
where
    PS: IntoIterator<Item = Service>,
    B: for<'a> Actor<'a, A, M>,
    PS: Send + Sync + 'static,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
{
    fn start_child(
        self: Box<Self>,
        system: System,
        input_arg: (),
    ) -> BoxedFuture<Result<ActorID, StartChildError>> {
        Box::pin(self.do_start_child(system))
    }
}

impl<B, A, M, PS> fmt::Debug for StartChildImpl<B, A, M, PS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartChild")
            .field("input_arg", &"()")
            .field("sup_id", &self.sup_id.to_string())
            .field("behavoiur", &std::any::type_name::<B>())
            .field("args", &std::any::type_name::<A>())
            .field("message", &std::any::type_name::<M>())
            .field("init_type", &self.init_type)
            .finish()
    }
}

impl<B, A, M, PS> StartChildImpl<B, A, M, PS>
where
    PS: IntoIterator<Item = Service>,
    B: for<'a> Actor<'a, A, M>,
    PS: Send + Sync + 'static,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
{
    async fn do_start_child(self: Box<Self>, system: System) -> Result<ActorID, StartChildError> {
        log::trace!("[{}|start_child] starting child [{:?}]", self.sup_id, self);

        let this = *self;

        let child_id = match this.init_type {
            InitType::NoAck => this.do_start_child_no_ack(system).await?,
            InitType::WithAck { init_timeout, stop_timeout: cancel_timeout } =>
                this.do_start_child_init_ack(system, init_timeout, cancel_timeout).await?,
        };

        Ok(child_id)
    }

    async fn do_start_child_no_ack(self, system: System) -> Result<ActorID, StartChildError> {
        let Self { sup_id, actor_behaviour, actor_args, provided_services, .. } = self;
        let spawn_opts = SpawnOpts::new().with_link(sup_id);
        let child_id = system.spawn(actor_behaviour, actor_args, spawn_opts).await?;

        let registrations = provided_services
            .into_iter()
            .map(|s| s.register(child_id))
            .collect::<Box<[_]>>();
        let registrations_count = registrations.len();

        system.add_data(child_id, registrations).await;

        log::trace!(
            "[{}|start_child_no_ack] started [child_id: {}, regs.len: {}]",
            sup_id,
            child_id,
            registrations_count
        );

        Ok(child_id)
    }
    async fn do_start_child_init_ack(
        self,
        system: System,
        init_timeout: Duration,
        cancel_timeout: Duration,
    ) -> Result<ActorID, StartChildError> {
        let Self { sup_id, actor_behaviour, actor_args, provided_services, .. } = self;
        let (init_ack_tx, init_ack_rx) = agner_actors::new_init_ack();
        let spawn_opts = SpawnOpts::new().with_init_ack(init_ack_tx);
        let intermediary_id = system.spawn(actor_behaviour, actor_args, spawn_opts).await?;

        let init_ack_result = init_ack_rx
            .timeout(init_timeout)
            .await
            .map(|id_opt| id_opt.ok_or(StartChildError::InitAckFailure))
            .err_flatten_in();

        match init_ack_result {
            Ok(child_id) => {
                system.link(sup_id, child_id).await;
                let registrations = provided_services
                    .into_iter()
                    .map(|s| s.register(child_id))
                    .collect::<Box<[_]>>();
                let registrations_count = registrations.len();
                system.add_data(child_id, registrations).await;

                log::trace!(
                    "[{}|start_child_init_ack] init-ack success [child_id: {}; regs.len: {}]",
                    sup_id,
                    child_id,
                    registrations_count
                );

                Ok(child_id)
            },
            Err(reason) => {
                log::warn!(
                    "[{}|start_child_init_ack] canceling init [error: {}]",
                    sup_id,
                    reason.pp()
                );

                if let Err(cancel_error) = util::try_exit(
                    system,
                    intermediary_id,
                    [(Exit::shutdown(), cancel_timeout), (Exit::kill(), cancel_timeout)],
                )
                .await
                {
                    log::error!("[{}|start_child_init_ack] failed to terminate intermediary [intermediary_id: {}, reason: {}]", sup_id, intermediary_id, cancel_error.as_ref().pp());
                }

                Err(reason)
            },
        }
    }
}

impl From<SysSpawnError> for StartChildError {
    fn from(e: SysSpawnError) -> Self {
        Self::SysSpawnError(e)
    }
}
impl From<tokio::time::error::Elapsed> for StartChildError {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(e)
    }
}