use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::process::Output;
use std::time::Duration;

use agner_actors::{Actor, ActorID, SpawnOpts, SysSpawnError, System};

use crate::service::Service;

#[cfg(test)]
mod tests;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

#[derive(Debug, thiserror::Error)]
pub enum StartChildError {
    #[error("System failed to spawn child")]
    SysSpawnError(#[source] SysSpawnError),
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
        log::trace!("[{}] starting child [{:?}]", self.sup_id, self);

        let this = *self;

        let child_id = match this.init_type {
            InitType::NoAck => this.do_start_child_no_ack(system).await?,
            InitType::WithAck { init_timeout, stop_timeout } =>
                this.do_start_child_init_ack(system).await?,
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

        system.add_data(child_id, registrations).await;

        Ok(child_id)
    }
    async fn do_start_child_init_ack(self, system: System) -> Result<ActorID, StartChildError> {
        unimplemented!()
    }
}

impl From<SysSpawnError> for StartChildError {
    fn from(_e: SysSpawnError) -> Self {
        unimplemented!()
    }
}
