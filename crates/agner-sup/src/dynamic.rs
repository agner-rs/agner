use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Duration;

use agner_actors::{
    Actor, ActorID, BoxError, Context, Event, ExitReason, Signal, SpawnOpts, System,
};
use futures::{stream, StreamExt};
use tokio::sync::oneshot;

pub type SpawnError = BoxError;

const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_MAX_PARALLELISM: usize = 32;

pub enum Message<IA> {
    StartChild(IA, oneshot::Sender<Result<ActorID, BoxError>>),
}

/// Start a child under the given sup
pub async fn start_child<IA>(system: &System, sup: ActorID, arg: IA) -> Result<ActorID, SpawnError>
where
    IA: Send + Sync + 'static,
{
    let (tx, rx) = oneshot::channel::<Result<ActorID, SpawnError>>();
    system.send(sup, Message::StartChild(arg, tx)).await;
    rx.await.map_err(SpawnError::from)?
}

#[derive(Debug, Clone)]
pub struct SupSpec<CS> {
    pub shutdown_timeout: Duration,
    pub child_spec: CS,
}

impl<CS> SupSpec<CS> {
    pub fn new(child_spec: CS) -> Self {
        Self { shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT, child_spec }
    }
    pub fn with_shutdown_timeout(self, shutdown_timeout: Duration) -> Self {
        Self { shutdown_timeout, ..self }
    }
}

/// Create a child-spec for a dynamic supervisor.
pub fn child_spec<B, AF, IA, OA, M>(behaviour: B, arg_factory: AF) -> impl ChildSpec<IA, M>
where
    B: for<'a> Actor<'a, OA, M> + Send + Sync + 'static,
    B: Clone,
    AF: FnMut(IA) -> OA,
    M: Send + Sync + Unpin + 'static,
    OA: Send + Sync + 'static,
{
    ChildSpecImpl { behaviour, arg_factory, _pd: Default::default() }
}

/// The behaviour function of a dynamic supervisor.
pub async fn dynamic_sup<CS, IA, M>(context: &mut Context<Message<IA>>, mut sup_spec: SupSpec<CS>)
where
    CS: ChildSpec<IA, M>,
    IA: Send + Sync + Unpin + 'static,
    M: Send + Sync + Unpin + 'static,
{
    context.trap_exit(true).await;
    context.init_ack(Default::default());

    let mut children = HashSet::new();
    loop {
        match context.next_event().await {
            Event::Message(Message::StartChild(arg, reply_to)) => {
                let (child_behaviour, child_arg) = sup_spec.child_spec.create(arg);
                let spawn_opts = SpawnOpts::new().with_link(context.actor_id());
                let spawn_result =
                    context.system().spawn(child_behaviour, child_arg, spawn_opts).await;
                let response = match spawn_result {
                    Ok(child_id) => {
                        children.insert(child_id);
                        Ok(child_id)
                    },
                    Err(reason) => Err(reason),
                };
                let _ = reply_to.send(response);
            },
            Event::Signal(Signal::Exit(myself, exit_reason)) if myself == context.actor_id() => {
                context.exit(exit_reason).await;
                unreachable!()
            },
            Event::Signal(Signal::Exit(terminated, exit_reason)) => {
                assert_ne!(terminated, context.actor_id());

                if !children.remove(&terminated) {
                    log::trace!(
                        "[{}] received SigExit from {} — {}",
                        context.actor_id(),
                        terminated,
                        exit_reason.pp()
                    );
                    let sup_exit_reason = ExitReason::Exited(terminated, exit_reason.into());
                    let child_exit_reason =
                        ExitReason::Exited(context.actor_id(), sup_exit_reason.to_owned().into());

                    log::trace!(
                        "[{}] shutting down {} children",
                        context.actor_id(),
                        children.len()
                    );

                    let children_shutdown_futures = children.drain().map(
                        |child_id| {
                            let sup_id = context.actor_id();
                            let system = context.system();
                            let child_exit_reason = child_exit_reason.to_owned();
                            let graceful_shutdown =
                                async move {
                                    system.exit(child_id, child_exit_reason).await;
                                    system.wait(child_id).await
                                };

                            let system = context.system();
                            let sure_shutdown =
                                async move {
                                    let graceful_shutdown_or_timeout = tokio::time::timeout(sup_spec.shutdown_timeout, graceful_shutdown);
                                    match graceful_shutdown_or_timeout.await {
                                        Ok(exit_reason) => log::trace!("[{}] child {} has gracefully exited: {}", sup_id, child_id, exit_reason),
                                        Err(_) => {
                                            log::warn!("[{}] child {} hasn't shut down gracefully on time. Killing it", sup_id, child_id);
                                            system.exit(child_id, ExitReason::Kill).await;
                                            system.wait(child_id).await;
                                        }
                                    }
                                };
                            sure_shutdown
                        });
                    let children_count = stream::iter(children_shutdown_futures)
                        .buffer_unordered(SHUTDOWN_MAX_PARALLELISM)
                        .count()
                        .await;

                    log::trace!(
                        "[{}] successfully shutdown {} children",
                        context.actor_id(),
                        children_count
                    );

                    context.exit(sup_exit_reason).await;
                    unreachable!()
                } else {
                    log::trace!(
                        "[{}] child {} terminated: {}",
                        context.actor_id(),
                        terminated,
                        exit_reason.pp()
                    );
                }
            },
        }
    }
}

pub trait ChildSpec<IA, M> {
    type Behavoiur: for<'a> Actor<'a, Self::Arg, M> + Send + Sync + 'static;

    type Arg: Send + Sync + 'static;

    fn create(&mut self, arg: IA) -> (Self::Behavoiur, Self::Arg);
}

struct ChildSpecImpl<B, M, IA, OA, AF> {
    behaviour: B,
    arg_factory: AF,
    _pd: PhantomData<(IA, OA, M)>,
}

impl<B, M, IA, OA, AF> ChildSpec<IA, M> for ChildSpecImpl<B, M, IA, OA, AF>
where
    B: for<'a> Actor<'a, OA, M> + Send + Sync + 'static,
    B: Clone,
    AF: FnMut(IA) -> OA,
    OA: Send + Sync + 'static,
{
    type Behavoiur = B;
    type Arg = OA;

    fn create(&mut self, arg: IA) -> (Self::Behavoiur, Self::Arg) {
        let arg = (self.arg_factory)(arg);
        (self.behaviour.clone(), arg)
    }
}
