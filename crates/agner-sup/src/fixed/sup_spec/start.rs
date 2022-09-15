use std::future::Future;
use std::pin::Pin;

use agner_actors::{ActorID, BoxError, Context, SpawnOpts};

use crate::fixed::hlist::index::{ApplyMut, OpMut};
use crate::fixed::hlist::HList;
use crate::fixed::{BoxedFuture, ChildSpec, SupSpec};

pub trait SupSpecStart<M> {
    fn start(
        &mut self,
        _context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, BoxError>>;
}

impl<M, R, CS> SupSpecStart<M> for SupSpec<R, CS>
where
    M: Send + Sync + 'static,
    CS: HList,
    for<'a> CS: ApplyMut<Start<'a, M>, BoxedFuture<Result<ActorID, BoxError>>>,
{
    fn start(
        &mut self,
        context: &mut Context<M>,
        index: usize,
    ) -> BoxedFuture<Result<ActorID, BoxError>> {
        let mut operation = Start { context };
        self.children.apply_mut(index, CS::LEN, &mut operation)
    }
}

struct Start<'a, M> {
    context: &'a mut Context<M>,
}

impl<'a, M, CS> OpMut<CS> for Start<'a, M>
where
    CS: ChildSpec,
{
    type Out = Pin<Box<dyn Future<Output = Result<ActorID, BoxError>>>>;

    fn apply_mut(&mut self, child_spec: &mut CS) -> Self::Out {
        let this_sup = self.context.actor_id();
        let system = self.context.system();
        let (behaviour, args) = child_spec.create();

        let spawn_future = async move {
            system.spawn(behaviour, args, SpawnOpts::new().with_link(this_sup)).await
        };

        Box::pin(spawn_future)
    }
}
