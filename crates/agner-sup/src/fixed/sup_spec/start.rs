use agner_actors::{ActorID, Context};

use crate::common::StartChildError;
use crate::fixed::hlist::index::{ApplyMut, OpMut};
use crate::fixed::hlist::HList;
use crate::fixed::{BoxedFuture, ChildSpec, SupSpec};

pub trait SupSpecStartChild<M> {
    fn start_child(
        &mut self,
        context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, StartChildError>>;
}

impl<M, R, CS> SupSpecStartChild<M> for SupSpec<R, CS>
where
    M: Send + Sync + 'static,
    CS: HList,
    for<'a> CS: ApplyMut<StartChild<'a, M>, BoxedFuture<Result<ActorID, StartChildError>>>,
{
    fn start_child(
        &mut self,
        context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, StartChildError>> {
        let mut operation = StartChild { context };
        self.children.apply_mut(index, CS::LEN, &mut operation)
    }
}

struct StartChild<'a, M> {
    context: &'a mut Context<M>,
}

impl<'a, M, CS> OpMut<CS> for StartChild<'a, M>
where
    CS: ChildSpec,
{
    type Out = BoxedFuture<Result<ActorID, StartChildError>>;

    fn apply_mut(&mut self, child_spec: &mut CS) -> Self::Out {
        let (behaviour, args) = child_spec.create();

        let regs = child_spec.regs().to_owned();

        let timeouts = Some((child_spec.init_timeout(), child_spec.stop_timeout()))
            .filter(|_| child_spec.init_ack());

        let start_fut = crate::common::start_child(
            self.context.system(),
            self.context.actor_id(),
            behaviour,
            args,
            timeouts,
            regs,
        );
        Box::pin(start_fut)
    }
}
