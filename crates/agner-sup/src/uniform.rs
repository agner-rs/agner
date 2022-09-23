use std::collections::HashSet;
use std::time::Duration;

use agner_actors::{ActorID, Context, Event, Exit, Never, Signal};
use agner_utils::std_error_pp::StdErrorPP;

use tokio::sync::oneshot;

use crate::common::{ProduceChild, StartChildError};

const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub enum Message<InArgs> {
    Start(InArgs, oneshot::Sender<Result<ActorID, StartChildError>>),
    Stop(ActorID, oneshot::Sender<Exit>),
    Noop,
}

#[derive(Debug, Clone)]
pub struct SupSpec<P> {
    shutdown_timeout: Duration,
    produce: P,
}

impl<P> SupSpec<P> {
    pub fn new(produce: P) -> Self {
        Self { shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT, produce }
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
    P: ProduceChild<A>,
    A: Unpin + Send + Sync + 'static,
{
    context.trap_exit(true).await;

    let SupSpec { shutdown_timeout, mut produce } = sup_spec;

    let mut shutting_down = false;
    let mut children: HashSet<ActorID> = Default::default();
    loop {
        match context.next_event().await {
            Event::Message(Message::Noop) => (),
            Event::Message(Message::Start(args, reply_to)) => {
                log::trace!("[{}] starting child", context.actor_id());

                let start_child = produce.produce(context.actor_id(), args);
                let result = start_child.start_child(context.system()).await;

                if let Some(actor_id) = result.as_ref().ok().copied() {
                    children.insert(actor_id);
                }

                log::trace!("[{}] start result {:?}", context.actor_id(), result);

                let _ = reply_to.send(result);
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
                            let _ = reply_to.send(exit);
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
                    let _ = reply_to.send(Exit::no_actor());
                },
            Event::Signal(Signal::Exit(actor_id, exit_reason)) =>
                if actor_id == context.actor_id() {
                    log::trace!(
                        "[{}] received a shutdown signal to myself. Shutting down",
                        context.actor_id()
                    );
                    shutting_down = true;

                    let system = context.system();
                    for actor_id in children.iter().copied() {
                        system.exit(actor_id, Exit::shutdown()).await;
                    }
                } else if children.remove(&actor_id) {
                    log::trace!(
                        "[{}] child {} terminated [exit: {}]",
                        context.actor_id(),
                        actor_id,
                        exit_reason.pp()
                    );
                    if shutting_down && children.is_empty() {
                        log::trace!(
                            "[{}] last child terminated. Shutting down",
                            context.actor_id()
                        );
                        context.exit(Exit::shutdown()).await;
                        unreachable!()
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

    use std::convert::Infallible;

    use agner_actors::System;
    use agner_utils::result_err_flatten::ResultErrFlattenIn;

    use crate::common::{args_factory, produce_child, InitType};

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
        let produce_worker = produce_child::new(
            worker,
            args_factory::map({
                let mut id = 0;
                move |name: &'static str| -> (usize, &'static str) {
                    id += 1;
                    (id, name)
                }
            }),
            InitType::NoAck,
            vec![],
        );
        let sup_spec = SupSpec::new(produce_worker);

        let system = System::new(Default::default());
        let sup = system.spawn(crate::uniform::run, sup_spec, Default::default()).await.unwrap();

        let (tx, rx) = oneshot::channel();
        system.send(sup, Message::<&'static str>::Start("one", tx)).await;
        let w1 = rx.await.err_flatten_in().unwrap();

        let (tx, rx) = oneshot::channel();
        system.send(sup, Message::<&'static str>::Start("two", tx)).await;
        let w2 = rx.await.err_flatten_in().unwrap();

        let (tx, rx) = oneshot::channel();
        system.send(sup, Message::<&'static str>::Start("three", tx)).await;
        let w3 = rx.await.err_flatten_in().unwrap();

        let (tx, rx) = oneshot::channel();
        system.send(sup, Message::<&'static str>::Stop(w1, tx)).await;
        assert!(rx.await.unwrap().is_shutdown());
        assert!(system.wait(w1).await.is_shutdown());

        system.exit(sup, Exit::shutdown()).await;
        assert!(system.wait(sup).await.is_shutdown());
        assert!(system.wait(w2).await.is_shutdown());
        assert!(system.wait(w3).await.is_shutdown());
    }
}
