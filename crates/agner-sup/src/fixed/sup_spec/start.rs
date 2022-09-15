use agner_actors::{Actor, ActorID, BoxError, Context, SpawnOpts, System};

use crate::fixed::hlist::index::{ApplyMut, OpMut};
use crate::fixed::hlist::HList;
use crate::fixed::{BoxedFuture, ChildSpec, SupSpec};
use crate::Registered;

pub trait SupSpecStartChild<M> {
    fn start_child(
        &mut self,
        _context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, BoxError>>;
}

impl<M, R, CS> SupSpecStartChild<M> for SupSpec<R, CS>
where
    M: Send + Sync + 'static,
    CS: HList,
    for<'a> CS: ApplyMut<StartChild<'a, M>, BoxedFuture<Result<ActorID, BoxError>>>,
{
    fn start_child(
        &mut self,
        context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, BoxError>> {
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
    type Out = BoxedFuture<Result<ActorID, BoxError>>;

    fn apply_mut(&mut self, child_spec: &mut CS) -> Self::Out {
        let this_sup = self.context.actor_id();
        let system = self.context.system();
        let (behaviour, args) = child_spec.create();
        let regs = child_spec.regs().to_owned();

        async fn start_child<B, A, M>(
            system: System,
            this_sup: ActorID,
            behaviour: B,
            args: A,
            regs: Vec<Registered>,
        ) -> Result<ActorID, BoxError>
        where
            B: for<'a> Actor<'a, A, M>,
            A: Send + Sync + 'static,
            M: Send + Sync + Unpin + 'static,
        {
            let child_id =
                system.spawn(behaviour, args, SpawnOpts::new().with_link(this_sup)).await?;

            for reg in regs {
                reg.update(child_id);
            }

            Ok(child_id)
        }

        Box::pin(start_child(system, this_sup, behaviour, args, regs))
    }
}
