use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Duration;

use agner_actors::{Actor, ActorID, Context, Event, Exit, Signal, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::std_error_pp::StdErrorPP;
use futures::{stream, StreamExt};
use tokio::sync::oneshot;

use crate::common::StartChildError;

const DEFAULT_INIT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_MAX_PARALLELISM: usize = 32;

pub enum Message<IA> {
    StartChild(IA, oneshot::Sender<Result<ActorID, StartChildError>>),
}

/// Start a child under the given sup
pub async fn start_child<IA>(
    system: &System,
    sup: ActorID,
    args: IA,
) -> Result<ActorID, StartChildError>
where
    IA: Send + Sync + 'static,
{
    let (tx, rx) = oneshot::channel::<Result<ActorID, StartChildError>>();
    system.send(sup, Message::StartChild(args, tx)).await;
    rx.await.map_err(StartChildError::Rx)?
}

#[derive(Debug, Clone)]
pub struct SupSpec<CS> {
    pub child_spec: CS,
}

impl<CS> SupSpec<CS> {
    pub fn new(child_spec: CS) -> Self {
        Self { child_spec }
    }
}

/// Create a child-spec for a dynamic supervisor.
pub fn child_spec<B, AF, IA, OA, M>(behaviour: B, args_factory: AF) -> impl ChildSpec<IA, M>
where
    B: for<'a> Actor<'a, OA, M> + Send + Sync + 'static,
    B: Clone,
    AF: FnMut(IA) -> OA,
    M: Send + Sync + Unpin + 'static,
    OA: Send + Sync + 'static,
{
    ChildSpecImpl {
        behaviour,
        args_factory,
        init_ack: true,
        init_timeout: DEFAULT_INIT_TIMEOUT,
        stop_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
        _pd: Default::default(),
    }
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
            Event::Message(Message::StartChild(args, reply_to)) => {
                let response =
                    match do_start_child(context, &mut sup_spec, args, &mut children).await {
                        Ok(child_id) => Ok(child_id),
                        Err(reason) => Err(reason),
                    };

                let _ = reply_to.send(response);
            },
            Event::Signal(Signal::Exit(terminated, exit_reason)) => {
                if !children.remove(&terminated) {
                    if terminated == context.actor_id() {
                        log::debug!(
                            "[{}] Shutdown requested — {}",
                            context.actor_id(),
                            exit_reason.pp()
                        );
                    } else {
                        log::debug!(
                            "[{}] Received SigExit from {} — {}",
                            context.actor_id(),
                            terminated,
                            exit_reason.pp()
                        );
                    };

                    let sup_exit_reason = Exit::exited(terminated, exit_reason);
                    let child_exit_reason =
                        Exit::exited(context.actor_id(), sup_exit_reason.to_owned());

                    log::trace!(
                        "[{}] shutting down {} children",
                        context.actor_id(),
                        children.len()
                    );

                    let children_shutdown_futures = children.drain().map(
                        |child_id| {
                            let sup_id = context.actor_id();
                            let system = context.system();
                            let child_stop_timeout = sup_spec.child_spec.stop_timeout();
                            let child_exit_reason = child_exit_reason.to_owned();
                            let graceful_shutdown =
                                async move {
                                    system.exit(child_id, child_exit_reason).await;
                                    system.wait(child_id).await
                                };

                            let system = context.system();
                            let sure_shutdown =
                                async move {
                                    let graceful_shutdown_or_timeout = graceful_shutdown.timeout(child_stop_timeout);
                                    match graceful_shutdown_or_timeout.await {
                                        Ok(exit_reason) => log::trace!("[{}] child {} has gracefully exited: {}", sup_id, child_id, exit_reason),
                                        Err(_) => {
                                            log::warn!("[{}] child {} hasn't shut down gracefully on time. Killing it", sup_id, child_id);
                                            system.exit(child_id, Exit::kill()).await;
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

                    log::debug!(
                        "[{}] successfully shutdown {} children. Exitting",
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
    type Behavoiur: for<'a> Actor<'a, Self::Args, M> + Send + Sync + 'static;

    type Args: Send + Sync + 'static;

    fn create(&mut self, args: IA) -> (Self::Behavoiur, Self::Args);
    fn with_init_ack(self) -> Self;
    fn without_init_ack(self) -> Self;
    fn init_ack(&self) -> bool;

    fn with_init_timeout(self, init_timeout: Duration) -> Self;
    fn init_timeout(&self) -> Duration;

    fn with_stop_timeout(self, stop_timeout: Duration) -> Self;
    fn stop_timeout(&self) -> Duration;
}

struct ChildSpecImpl<B, M, IA, OA, AF> {
    behaviour: B,
    args_factory: AF,
    init_ack: bool,
    init_timeout: Duration,
    stop_timeout: Duration,
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
    type Args = OA;

    fn create(&mut self, args: IA) -> (Self::Behavoiur, Self::Args) {
        let args = (self.args_factory)(args);
        (self.behaviour.clone(), args)
    }
    fn init_ack(&self) -> bool {
        self.init_ack
    }
    fn with_init_ack(self) -> Self {
        Self { init_ack: true, ..self }
    }
    fn without_init_ack(self) -> Self {
        Self { init_ack: false, ..self }
    }

    fn with_init_timeout(self, init_timeout: Duration) -> Self {
        Self { init_timeout, ..self }
    }
    fn init_timeout(&self) -> Duration {
        self.init_timeout
    }

    fn with_stop_timeout(self, stop_timeout: Duration) -> Self {
        Self { stop_timeout, ..self }
    }
    fn stop_timeout(&self) -> Duration {
        self.stop_timeout
    }
}

async fn do_start_child<CS, IA, M>(
    context: &mut Context<Message<IA>>,
    sup_spec: &mut SupSpec<CS>,
    args: IA,
    children: &mut HashSet<ActorID>,
) -> Result<ActorID, StartChildError>
where
    CS: ChildSpec<IA, M>,
    IA: Send + Sync + Unpin + 'static,
    M: Send + Sync + Unpin + 'static,
{
    let (child_behaviour, child_args) = sup_spec.child_spec.create(args);
    let init_timeouts =
        Some((sup_spec.child_spec.init_timeout(), sup_spec.child_spec.stop_timeout()))
            .filter(|_| sup_spec.child_spec.init_ack());

    let child_id = crate::common::start_child(
        context.system(),
        context.actor_id(),
        child_behaviour,
        child_args,
        init_timeouts,
        [],
    )
    .await?;
    log::trace!("[{}] adding {} to children", context.actor_id(), child_id);
    children.insert(child_id);

    Ok(child_id)
}
