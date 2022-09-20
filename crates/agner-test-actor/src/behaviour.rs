use std::convert::Infallible;
use std::sync::Arc;

use agner_actors::Context;
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::exited::Exited;
use crate::query::{ExitRq, InitAckRq, NextEventRq, Query, SetLinkRq, SetTrapExitRq};
use crate::registry::TestActorEntry;
use crate::TestActorRegistry;

pub struct Args<M> {
    pub registry: TestActorRegistry,
    pub ack_tx: oneshot::Sender<Infallible>,
    pub ctl_rx: mpsc::UnboundedReceiver<Query<M>>,
    pub ctl_tx: mpsc::UnboundedSender<Query<M>>,
}

pub async fn run<M>(context: &mut Context<M>, args: Args<M>)
where
    M: Send + Sync + Unpin + 'static,
{
    let Args { ack_tx, mut ctl_rx, ctl_tx, registry } = args;
    std::mem::drop(ack_tx);

    let exited = tokio::spawn({
        let system = context.system();
        let actor_id = context.actor_id();
        async move { system.wait(actor_id).await }
    });

    let entry = TestActorEntry {
        system: context.system(),
        ctl_tx: Box::new(ctl_tx),
        exited: Arc::new(Mutex::new(Exited::Wait(exited))),
    };
    registry.0.write().await.insert(context.actor_id(), entry);

    while let Some(query) = ctl_rx.recv().await {
        match query {
            Query::Exit(ExitRq { reason, .. }) => {
                log::trace!("[{}] exitting: {}", context.actor_id(), reason);
                context.exit(reason).await;
            },
            Query::SetTrapExit(SetTrapExitRq { set_to, .. }) => {
                log::trace!("[{}] setting trap_exit={}", context.actor_id(), set_to);
                context.trap_exit(set_to).await;
            },
            Query::NextEvent(NextEventRq { timeout, reply_to }) => {
                if let Ok(event) = context.next_event().timeout(timeout).await {
                    let _ = reply_to.send(event);
                }
            },
            Query::SetLink(SetLinkRq { actor, link, .. }) =>
                if link {
                    context.link(actor).await;
                } else {
                    context.unlink(actor).await;
                },
            Query::InitAck(InitAckRq { value, .. }) => {
                context.init_ack(value);
            },
        }
    }
}
