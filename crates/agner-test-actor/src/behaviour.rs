use agner_actors::Context;
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use tokio::sync::mpsc;

use crate::query::{ExitRq, NextEventRq, Query, SetLinkRq, SetTrapExitRq};

pub struct Args<M> {
    pub ctl_rx: mpsc::UnboundedReceiver<Query<M>>,
}

pub async fn run<M>(context: &mut Context<M>, args: Args<M>)
where
    M: Send + Sync + Unpin + 'static,
{
    let mut ctl_rx = args.ctl_rx;
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
        }
    }
}
