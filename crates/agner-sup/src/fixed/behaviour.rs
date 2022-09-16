use agner_actors::{BoxError, Context, Event, Signal};

use crate::fixed::hlist::HList;
use crate::fixed::restart_strategy::{Action, Instant};
use crate::fixed::{Decider, SupSpec};

use crate::fixed::sup_spec::SupSpecStartChild;

use super::sup_spec::SupSpecStopChild;
use super::RestartStrategy;

#[derive(Debug)]
pub enum Message {}

pub async fn fixed_sup<R, CS>(
    context: &mut Context<Message>,
    mut sup_spec: SupSpec<R, CS>,
) -> Result<(), BoxError>
where
    CS: HList,
    R: RestartStrategy,
    SupSpec<R, CS>: SupSpecStartChild<Message>,
    SupSpec<R, CS>: SupSpecStopChild<Message>,
{
    context.trap_exit(true).await;
    context.init_ack(Default::default());

    log::trace!("[{}] starting fixed sup with {} children", context.actor_id(), CS::LEN);

    let mut children = Vec::with_capacity(CS::LEN);

    for child_idx in 0..CS::LEN {
        log::trace!("[{}] initially starting child #{}...", context.actor_id(), child_idx);
        let child_id = sup_spec.start_child(context, child_idx).await?;
        children.push(child_id);
        log::trace!("[{}]   child #{}: {}", context.actor_id(), child_idx, child_id);
    }
    let children = children;
    assert_eq!(children.len(), CS::LEN);

    log::trace!(
        "[{}] initializing restart decider for {}",
        context.actor_id(),
        sup_spec.restart_strategy
    );
    let mut restart_decider =
        sup_spec.restart_strategy.new_decider(context.actor_id(), &children[..]);
    std::mem::drop(children);

    loop {
        if let Some(action) = restart_decider.next_action() {
            match action {
                Action::Exit(exit_reason) => {
                    context.exit(exit_reason).await;
                    unreachable!()
                },
                Action::Start(child_idx) => {
                    log::trace!("[{}] starting child #{}...", context.actor_id(), child_idx);
                    let child_id = sup_spec.start_child(context, child_idx).await?;
                    log::trace!("[{}]   child #{}: {}", context.actor_id(), child_idx, child_id);
                },
                Action::Stop(child_idx, actor_id, exit_reason) => {
                    log::trace!(
                        "[{}] stopping child #{} ({})...",
                        context.actor_id(),
                        child_idx,
                        actor_id
                    );
                    sup_spec.stop_child(context, child_idx, actor_id, exit_reason).await?;
                    log::trace!(
                        "[{}]   child #{} ({}) stopped",
                        context.actor_id(),
                        child_idx,
                        actor_id
                    );
                },
            }
        }

        let event = context.next_event().await;

        match event {
            Event::Message(message) => unimplemented!("message: {:?}", message),
            Event::Signal(Signal::Exit(actor_id, exit_reason)) =>
                restart_decider.actor_down(Instant::now(), actor_id, exit_reason),
        }
    }
}
