use std::collections::HashMap;
use std::time::{Duration, Instant};

use agner_actors::{ActorID, Context, Event, Exit, Never, Signal};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use agner_utils::std_error_pp::StdErrorPP;
use tokio::sync::oneshot;

use crate::common::{ProduceChild, StartChildError};
use crate::mixed::child_id::ChildID;
use crate::mixed::restart_strategy::{Action, Decider, RestartStrategy};
use crate::mixed::sup_spec::SupSpec;
use crate::mixed::ChildSpec;

#[derive(Debug)]
pub enum Message<ID> {
    RmChild(ID, oneshot::Sender<Result<(), SupervisorError>>),
    AddChild(ChildSpec<ID>, oneshot::Sender<Result<ActorID, SupervisorError>>),
    WhichChildren(oneshot::Sender<Vec<(ID, ActorID)>>),
}

pub async fn run<ID, RS>(
    context: &mut Context<Message<ID>>,
    sup_spec: SupSpec<ID, RS>,
) -> Result<Never, Exit>
where
    ID: ChildID,
    RS: RestartStrategy<ID>,
    RS::Decider: Decider<ID, Duration, Instant>,
{
    context.trap_exit(true).await;

    log::trace!(
        "[{}] initializing decider [restart-strategy: {:?}]",
        context.actor_id(),
        sup_spec.restart_strategy
    );
    let SupSpec { restart_strategy, children } = sup_spec;
    let mut decider = restart_strategy.new_decider(context.actor_id());
    let mut child_ids: Vec<ID> = vec![];
    let mut child_actors: HashMap<ID, ActorID> = Default::default();
    let mut child_specs: HashMap<ID, ChildSpec<ID>> = Default::default();

    for child_spec in children {
        decider.add_child(child_spec.id, child_spec.child_type).map_err(Exit::custom)?;
        child_ids.push(child_spec.id);
        assert!(child_specs.insert(child_spec.id, child_spec).is_none());
    }

    let mut decider_has_actions = true;
    loop {
        let mut first_context_poll = true;

        loop {
            log::trace!(
                "[{}] before invoking next_event [decider-has-actions: {}, first-context-poll: {}]",
                context.actor_id(),
                decider_has_actions,
                first_context_poll
            );

            let next_event_opt = if decider_has_actions || !first_context_poll {
                context.next_event().timeout(Duration::ZERO).await.ok()
            } else {
                Some(context.next_event().await)
            };

            first_context_poll = false;

            if let Some(next_event) = next_event_opt {
                match next_event {
                    Event::Message(message) =>
                        handle_message(context, &mut decider, &mut child_specs, message).await?,
                    Event::Signal(signal) => handle_signal(context, &mut decider, signal).await?,
                }
            } else {
                break
            }
        }

        decider_has_actions = match decider.next_action().map_err(Exit::custom)? {
            None => false,
            Some(action) => {
                process_action(context, &mut decider, &mut child_specs, &mut child_actors, action)
                    .await?;
                true
            },
        };
    }
}

async fn handle_signal<ID, D>(
    _context: &mut Context<Message<ID>>,
    decider: &mut D,
    signal: Signal,
) -> Result<(), Exit>
where
    ID: ChildID,
    D: Decider<ID, Duration, Instant>,
{
    match signal {
        Signal::Exit(actor_id, exit_reason) => {
            decider
                .exit_signal(actor_id, exit_reason, Instant::now())
                .map_err(Exit::custom)?;
            Ok(())
        },
    }
}

async fn handle_message<ID, D>(
    _context: &mut Context<Message<ID>>,
    _decider: &mut D,
    _child_specs: &mut HashMap<ID, ChildSpec<ID>>,
    message: Message<ID>,
) -> Result<(), Exit> {
    match message {
        Message::WhichChildren(_) => unimplemented!(),
        Message::RmChild(_, _) => unimplemented!(),
        Message::AddChild(_, _) => unimplemented!(),
    }
}

async fn process_action<ID, D>(
    context: &mut Context<Message<ID>>,
    decider: &mut D,
    child_specs: &mut HashMap<ID, ChildSpec<ID>>,
    child_actors: &mut HashMap<ID, ActorID>,
    action: Action<ID>,
) -> Result<(), Exit>
where
    ID: ChildID,
    D: Decider<ID, Duration, Instant>,
{
    match action {
        Action::Shutdown(reason) => {
            log::trace!(
                "[{}] decider requested shutdown [exit: {}]",
                context.actor_id(),
                reason.pp()
            );
            context.exit(reason).await;
            unreachable!()
        },
        Action::Start(child_id) => {
            log::trace!("[{}] starting child[{:?}]", context.actor_id(), child_id);

            if let Some(child_spec) = child_specs.get_mut(&child_id) {
                let start_child = child_spec.produce.produce(context.actor_id(), ());
                let actor_id = start_child
                    .start_child(context.system())
                    .await
                    .map_err(SupervisorError::StartChildFailure)
                    .map_err(Exit::custom)?;
                child_actors.insert(child_id, actor_id);
                decider.child_started(child_id, actor_id).map_err(Exit::custom)?;
            } else {
                return Err(Exit::custom(SupervisorError::UnknownId))
            }
        },
        Action::Stop(child_id) => {
            log::trace!("[{}] stopping child[{:?}]", context.actor_id(), child_id);

            if let Some((actor_id, child_spec)) =
                child_actors.remove(&child_id).zip(child_specs.get(&child_id))
            {
                log::trace!("[{}] stopping child[{:?}]", context.actor_id(), actor_id);
                crate::common::util::try_exit(
                    context.system(),
                    actor_id,
                    child_spec.shutdown.to_owned(),
                )
                .await
                .map_err(Exit::custom)?;
            } else {
                return Err(Exit::custom(SupervisorError::UnknownId))
            }
        },
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("Decider returned unknown id")]
    UnknownId,

    #[error("Failed to start child")]
    StartChildFailure(#[source] StartChildError),
}
