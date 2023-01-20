//! Uniform Supervisor
//! =======

use std::collections::HashSet;
use std::sync::Arc;

use agner_actors::{ActorID, Context, Event, Exit, Never, Signal, System};
use agner_init_ack::ContextInitAckExt;
use agner_utils::result_err_flatten::ResultErrFlattenIn;
use agner_utils::std_error_pp::StdErrorPP;

use tokio::sync::oneshot;

use crate::common::{CreateChild, StartChildError};

mod child_spec;
pub use child_spec::UniformChildSpec;

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
    A: Send + 'static,
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
    A: Send + 'static,
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
pub struct SupSpec<CS>(CS);

impl<CS> SupSpec<CS> {
    pub fn new(child_spec: CS) -> Self {
        Self(child_spec)
    }
}

/// The behaviour function of the [Uniform Supervisor](crate::uniform).
pub async fn run<SupArg, B, A, M>(
    context: &mut Context<Message<SupArg>>,
    sup_spec: SupSpec<UniformChildSpec<B, A, M>>,
) -> Result<Never, Exit>
where
    UniformChildSpec<B, A, M>: CreateChild<Args = SupArg>,
    SupArg: Unpin + Send + 'static,
    B: Send + Sync + 'static,
    A: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    context.trap_exit(true).await;
    context.init_ack_ok(Default::default());

    let SupSpec(mut child_spec) = sup_spec;

    let mut shutting_down = None;
    let mut children: HashSet<ActorID> = Default::default();
    loop {
        match context.next_event().await {
            Event::Message(Message::Noop) => (),
            Event::Message(Message::Start(args, reply_to)) => {
                tracing::trace!("starting child");

                let result =
                    child_spec.create_child(&context.system(), context.actor_id(), args).await;

                if let Some(actor_id) = result.as_ref().ok().copied() {
                    children.insert(actor_id);
                }

                tracing::trace!("start result {:?}", result);

                let _ = reply_to.send(result.map_err(Into::into));
            },
            Event::Message(Message::Stop(actor_id, reply_to)) =>
                if children.contains(&actor_id) {
                    tracing::trace!("stopping child {}", actor_id);

                    let system = context.system();
                    let job = {
                        let shutdown_sequence = child_spec.shutdown_sequence().to_owned();
                        async move {
                            tracing::trace!("stop-job enter [child: {}]", actor_id);
                            let result =
                                crate::common::stop_child(system, actor_id, shutdown_sequence)
                                    .await;

                            tracing::trace!(
                                "stop-job done [child: {}; result: {:?}]",
                                actor_id,
                                result
                            );

                            if let Ok(exit) = result {
                                let _ = reply_to.send(Ok(exit));
                            }
                            Message::<SupArg>::Noop
                        }
                    };
                    context.future_to_inbox(job).await;
                } else {
                    tracing::trace!(
                        "received a request to stop an unknown actor ({}). Ignoring.",
                        actor_id
                    );
                    let _ = reply_to.send(Ok(Exit::no_actor()));
                },
            Event::Signal(Signal::Exit(actor_id, exit_reason)) =>
                if actor_id == context.actor_id() {
                    tracing::trace!("received a shutdown signal to myself. Shutting down",);
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
                    tracing::trace!("child {} terminated [exit: {}]", actor_id, exit_reason.pp());
                    if children.is_empty() {
                        if let Some(exit_reason) = shutting_down {
                            tracing::trace!(
                                "last child terminated. Shutting down: {}",
                                exit_reason.pp()
                            );
                            context.exit(exit_reason).await;
                            unreachable!()
                        }
                    }
                } else {
                    tracing::trace!(
                        "unknown linked process ({}) termianted. Shutting down [exit: {}]",
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
    use std::time::Duration;

    use agner_actors::System;

    use crate::common::InitType;

    #[tokio::test]
    async fn ergonomics() {
        async fn worker(
            _context: &mut Context<Infallible>,
            (worker_id, worker_name): (usize, &'static str),
        ) -> Result<Never, Exit> {
            tracing::info!("worker [id: {:?}, name: {:?}]", worker_id, worker_name);
            tokio::time::sleep(Duration::from_secs(3)).await;
            std::future::pending().await
        }
        let child_spec = UniformChildSpec::uniform()
            .behaviour(worker)
            .args_call1({
                let mut id = 0;
                move |name| {
                    id += 1;
                    (id, name)
                }
            })
            .init_type(InitType::no_ack());

        let sup_spec = SupSpec::new(child_spec);

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
