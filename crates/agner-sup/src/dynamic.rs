use std::collections::HashSet;
use std::marker::PhantomData;

use agner_actors::{
    Actor, ActorID, BoxError, Context, Event, ExitReason, Signal, SpawnOpts, System,
};
use tokio::sync::oneshot;

pub type SpawnError = BoxError;

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
pub async fn dynamic_sup<CS, IA, M>(context: &mut Context<Message<IA>>, mut child_spec: CS)
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
                let (child_behaviour, child_arg) = child_spec.create(arg);
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

            Event::Signal(Signal::Exit(terminated, exit_reason)) => {
                if !children.remove(&terminated) {
                    context.exit(ExitReason::Exited(terminated, exit_reason.into())).await;
                    unreachable!()
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
