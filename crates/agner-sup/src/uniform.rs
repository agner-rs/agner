use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use agner_actors::{ActorID, Context, Event, Exit, Never, Signal, System};
use agner_init_ack::ContextInitAckExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;

use tokio::sync::oneshot;

use crate::common::{ChildFactory, StartChildError};

const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, thiserror::Error)]
pub enum SupervisorError {
    #[error("Failed to start a child")]
    StartChildError(#[source] StartChildError),

    #[error("oneshot-rx error")]
    OneshotRx(#[source] oneshot::error::RecvError),

    #[error("Timeout")]
    Timeout(#[source] Arc<tokio::time::error::Elapsed>),
}

pub async fn start_child<A>(
    system: &System,
    sup: ActorID,
    args: A,
) -> Result<ActorID, SupervisorError>
where
    A: Send + Sync + 'static,
{
    let (tx, rx) = oneshot::channel();
    system.send(sup, Message::Start(args, tx)).await;
    rx.await.err_flatten_in()
}

pub async fn stop_child<A>(
    system: &System,
    sup: ActorID,
    child: ActorID,
) -> Result<Exit, SupervisorError>
where
    A: Send + Sync + 'static,
{
    let (tx, rx) = oneshot::channel();
    system.send(sup, Message::<A>::Stop(child, tx)).await;
    rx.await.err_flatten_in()
}

pub enum Message<InArgs> {
    Start(InArgs, oneshot::Sender<Result<ActorID, SupervisorError>>),
    Stop(ActorID, oneshot::Sender<Result<Exit, SupervisorError>>),
    Noop,
}

#[derive(Debug, Clone)]
pub struct SupSpec<P> {
    shutdown_timeout: Duration,
    child_factory: P,
}

impl<P> SupSpec<P> {
    pub fn new(child_factory: P) -> Self {
        Self { shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT, child_factory }
    }
    pub fn with_shutdown_timeout(self, shutdown_timeout: Duration) -> Self {
        Self { shutdown_timeout, ..self }
    }
}

pub async fn run<P, A>(
    context: &mut Context<Message<A>>,
    sup_spec: SupSpec<P>,
) -> Result<Never, Exit>
where
    P: ChildFactory<A>,
    A: Unpin + Send + Sync + 'static,
{
    context.trap_exit(true).await;
    context.init_ack_ok(Default::default());

    let SupSpec { shutdown_timeout, mut child_factory } = sup_spec;

    let mut shutting_down = None;
    let mut children: HashSet<ActorID> = Default::default();
    loop {
        match context.next_event().await {
            Event::Message(Message::Noop) => (),
            Event::Message(Message::Start(args, reply_to)) => {
                log::trace!("[{}] starting child", context.actor_id());

                let result = child_factory.produce(context.system(), context.actor_id(), args).await;

                if let Some(actor_id) = result.as_ref().ok().copied() {
                    children.insert(actor_id);
                }

                log::trace!("[{}] start result {:?}", context.actor_id(), result);

                let _ = reply_to.send(result.map_err(Into::into));
            },
            Event::Message(Message::Stop(actor_id, reply_to)) =>
                if children.contains(&actor_id) {
                    log::trace!("[{}] stopping child {}", context.actor_id(), actor_id);

                    let system = context.system();
                    let sup_id = context.actor_id();
                    let job = async move {
                        log::trace!("[{}] stop-job enter [child: {}]", sup_id, actor_id);
                        let result = crate::common::util::try_exit(
                            system,
                            actor_id,
                            [
                                (Exit::shutdown(), shutdown_timeout),
                                (Exit::kill(), shutdown_timeout),
                            ],
                        )
                        .await;

                        log::trace!(
                            "[{}] stop-job done [child: {}; result: {:?}]",
                            sup_id,
                            actor_id,
                            result
                        );

                        if let Ok(exit) = result {
                            let _ = reply_to.send(Ok(exit));
                        }
                        Message::<A>::Noop
                    };
                    context.future_to_inbox(job).await;
                } else {
                    log::trace!(
                        "[{}] received a request to stop an unknown actor ({}). Ignoring.",
                        context.actor_id(),
                        actor_id
                    );
                    let _ = reply_to.send(Ok(Exit::no_actor()));
                },
            Event::Signal(Signal::Exit(actor_id, exit_reason)) =>
                if actor_id == context.actor_id() {
                    log::trace!(
                        "[{}] received a shutdown signal to myself. Shutting down",
                        context.actor_id()
                    );
                    shutting_down = Some(exit_reason.to_owned());

                    let system = context.system();
                    let mut has_some_children = false;
                    for actor_id in children.iter().copied() {
                        has_some_children = true;

                        system.exit(actor_id, Exit::shutdown()).await;
                    }

                    if !has_some_children {
                        context.exit(exit_reason).await;
                        unreachable!()
                    }
                } else if children.remove(&actor_id) {
                    log::trace!(
                        "[{}] child {} terminated [exit: {}]",
                        context.actor_id(),
                        actor_id,
                        exit_reason.pp()
                    );
                    if children.is_empty() {
                        if let Some(exit_reason) = shutting_down {
                            log::trace!(
                                "[{}] last child terminated. Shutting down: {}",
                                context.actor_id(),
                                exit_reason.pp()
                            );
                            context.exit(exit_reason).await;
                            unreachable!()
                        }
                    }
                } else {
                    log::trace!(
                        "[{}] unknown linked process ({}) termianted. Shutting down [exit: {}]",
                        context.actor_id(),
                        actor_id,
                        exit_reason.pp()
                    );
                    context.exit(Exit::linked(actor_id, exit_reason)).await;
                    unreachable!()
                },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::StreamExt;
    use std::convert::Infallible;

    use agner_actors::System;

    use crate::common::{args_factory, child_factory, InitType};

    #[tokio::test]
    async fn ergonomics() {
        let _ = dotenv::dotenv();
        let _ = pretty_env_logger::try_init_timed();

        async fn worker(
            context: &mut Context<Infallible>,
            (worker_id, worker_name): (usize, &'static str),
        ) -> Result<Never, Exit> {
            log::info!(
                "[{}] worker [id: {:?}, name: {:?}]",
                context.actor_id(),
                worker_id,
                worker_name
            );
            tokio::time::sleep(Duration::from_secs(3)).await;
            std::future::pending().await
        }
        let produce_worker = child_factory::new(
            worker,
            args_factory::map({
                let mut id = 0;
                move |name: &'static str| -> (usize, &'static str) {
                    id += 1;
                    (id, name)
                }
            }),
            InitType::NoAck,
        );
        let sup_spec = SupSpec::new(produce_worker);

        let system = System::new(Default::default());
        let sup = system.spawn(crate::uniform::run, sup_spec, Default::default()).await.unwrap();

        let w1 = start_child(&system, sup, "one").await.unwrap();
        let w2 = start_child(&system, sup, "two").await.unwrap();
        let w3 = start_child(&system, sup, "three").await.unwrap();

        let w1_exited = stop_child::<&str>(&system, sup, w1).await.unwrap();
        assert!(w1_exited.is_shutdown());
        assert!(system.wait(w1).await.is_shutdown());

        system.exit(sup, Exit::shutdown()).await;
        assert!(system.wait(sup).await.is_shutdown());
        assert!(system.wait(w2).await.is_shutdown());
        assert!(system.wait(w3).await.is_shutdown());

        assert!(system.all_actors().collect::<Vec<_>>().await.is_empty());
    }
}

impl From<oneshot::error::RecvError> for SupervisorError {
    fn from(e: oneshot::error::RecvError) -> Self {
        Self::OneshotRx(e)
    }
}
impl From<StartChildError> for SupervisorError {
    fn from(e: StartChildError) -> Self {
        Self::StartChildError(e)
    }
}
impl From<tokio::time::error::Elapsed> for SupervisorError {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(Arc::new(e))
    }
}
