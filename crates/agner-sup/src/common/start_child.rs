use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::process::Output;
use std::time::Duration;

use agner_actors::{Actor, ActorID, SysSpawnError, System};

use crate::service::Service;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

#[derive(Debug, thiserror::Error)]
pub enum StartChildError {}

pub trait StartChild<A>: Send + Sync + 'static {
    fn start_child(
        self: Box<Self>,
        system: System,
    ) -> BoxedFuture<Result<ActorID, StartChildError>>;
}

#[derive(Debug, Clone, Copy)]
pub enum InitType {
    NoAck,
    WithAck { init_timeout: Duration, stop_timeout: Duration },
}

#[derive(Debug)]
struct StartChildImpl<PS, B, A, M> {
    system: System,
    sup_id: ActorID,
    provided_services: PS,
    actor_behaviour: B,
    actor_args: A,
    actor_message: PhantomData<M>,
    init_type: InitType,
}

impl<PS, B, A, M> StartChild<()> for StartChildImpl<PS, B, A, M>
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
    ) -> BoxedFuture<Result<ActorID, StartChildError>> {
        Box::pin(self.do_start_child(system))
    }
}

impl<PS, B, A, M> StartChildImpl<PS, B, A, M>
where
    PS: IntoIterator<Item = Service>,
    B: for<'a> Actor<'a, A, M>,
    PS: Send + Sync + 'static,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
{
    async fn do_start_child(self: Box<Self>, system: System) -> Result<ActorID, StartChildError> {
        let Self { actor_behaviour, actor_args, .. } = *self;
        let child_id = system.spawn(actor_behaviour, actor_args, Default::default()).await?;

        Ok(child_id)
    }
}

impl From<SysSpawnError> for StartChildError {
    fn from(_e: SysSpawnError) -> Self {
        unimplemented!()
    }
}
