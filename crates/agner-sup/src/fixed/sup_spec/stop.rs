use agner_actors::{ActorID, Context, Exit};

use crate::common::StopChildError;
use crate::fixed::hlist::index::{ApplyMut, OpMut};
use crate::fixed::hlist::HList;
use crate::fixed::{BoxedFuture, ChildSpec, SupSpec};

pub trait SupSpecStopChild<M> {
    fn stop_child(
        &mut self,
        context: &mut Context<M>,
        index: usize,
        actor_id: ActorID,
        exit_reason: Exit,
    ) -> BoxedFuture<Result<(), StopChildError>>;
}

impl<M, R, CS> SupSpecStopChild<M> for SupSpec<R, CS>
where
    M: Send + Sync + 'static,
    CS: HList,
    for<'a> CS: ApplyMut<StopChild<'a, M>, BoxedFuture<Result<(), StopChildError>>>,
{
    fn stop_child(
        &mut self,
        context: &mut Context<M>,
        index: usize,
        actor_id: ActorID,
        exit_reason: Exit,
    ) -> BoxedFuture<Result<(), StopChildError>> {
        let mut operation = StopChild { context, actor_id, exit_reason };
        self.children.apply_mut(index, CS::LEN, &mut operation)
    }
}

struct StopChild<'a, M> {
    context: &'a mut Context<M>,
    actor_id: ActorID,
    exit_reason: Exit,
}

impl<'a, M, CS> OpMut<CS> for StopChild<'a, M>
where
    CS: ChildSpec,
{
    type Out = BoxedFuture<Result<(), StopChildError>>;

    fn apply_mut(&mut self, child_spec: &mut CS) -> Self::Out {
        let stop_fut = crate::common::stop_child(
            self.context.system(),
            self.context.actor_id(),
            self.actor_id,
            child_spec.stop_timeout(),
            self.exit_reason.to_owned(),
        );

        Box::pin(stop_fut)
    }
}
