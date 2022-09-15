use std::time::Duration;

use agner_actors::{Actor, ActorID, BoxError, Context, ExitReason, SpawnOpts, System};

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
        let timeout = child_spec.init_timeout();

        async fn start_child<B, A, M>(
            system: System,
            this_sup: ActorID,
            behaviour: B,
            args: A,
            regs: Vec<Registered>,
            timeout: Duration,
        ) -> Result<ActorID, BoxError>
        where
            B: for<'a> Actor<'a, A, M>,
            A: Send + Sync + 'static,
            M: Send + Sync + Unpin + 'static,
        {
            let (init_ack_tx, init_ack_rx) = agner_actors::new_init_ack();
            let child_id = system
                .spawn(
                    behaviour,
                    args,
                    SpawnOpts::new().with_link(this_sup).with_init_ack(init_ack_tx),
                )
                .await?;

            let reported_id_result = match tokio::time::timeout(timeout, init_ack_rx).await {
                Ok(Some(id)) => Ok(id),
                Ok(None) => Err("init-ack channel hangup"),
                Err(_) => Err("init-ack timeout"),
            };
            if reported_id_result.is_err() {
                system
                    .exit(
                        child_id,
                        ExitReason::Shutdown(Some(BoxError::from("init-ack timeout").into())),
                    )
                    .await;
            }
            let reported_id = reported_id_result?;

            for reg in regs {
                reg.update(reported_id);
            }

            Ok(reported_id)
        }

        Box::pin(start_child(system, this_sup, behaviour, args, regs, timeout))
    }
}
