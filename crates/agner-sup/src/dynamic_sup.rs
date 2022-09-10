use std::collections::{HashMap, HashSet};

use agner_actor::error::{ExitReason, WellKnownReason};
use agner_actor::{oneshot, Actor, ActorID, Context, Message, Signal, StartOpts, System};

mod error;
pub use error::{StartError, StopError, SupError};

mod request;
pub use request::SupRq;

mod seed_factory;
pub use seed_factory::SeedFactory;

mod dyn_sup_seed;
pub use dyn_sup_seed::DynSupSeed;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct DynamicSup<WA, S, Rq> {
	_pd: std::marker::PhantomData<(WA, Rq)>,
	seed: S,
	children: HashSet<ActorID>,
	exitting: HashMap<ActorID, oneshot::Tx<Result<(), StopError>>>,
}

#[async_trait::async_trait]
impl<Sys, WA, S, Rq> Actor<Sys> for DynamicSup<WA, S, Rq>
where
	Sys: System,
	WA: Actor<Sys>,
	S: DynSupSeed<Rq, WA::Seed>,
	Rq: Message,
{
	type Seed = S;
	type Message = SupRq<Rq>;

	async fn init<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		let seed = context.seed_mut().take();
		Ok(Self {
			_pd: Default::default(),
			seed,
			children: Default::default(),
			exitting: Default::default(),
		})
	}

	async fn handle_message<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		match message {
			SupRq::StartChild(rq, reply_to) => self
				.handle_start_child(context, reply_to, rq)
				.await
				.map_err(ExitReason::error_actor),
			SupRq::StopChild(child_id, reply_to) => self
				.handle_stop_child(context, reply_to, child_id)
				.await
				.map_err(ExitReason::error_actor),
		}
	}

	async fn handle_signal<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		signal: Signal,
	) -> Result<(), ExitReason> {
		match signal {
			Signal::Exit(offender, _) =>
				if let Some(reply_to) = self.exitting.remove(&offender) {
					reply_to.send(Ok(()));
					Ok(())
				} else if self.children.remove(&offender) {
					Ok(())
				} else {
					context.exit(WellKnownReason::linked_exit(offender).into()).await;
					unreachable!()
				},
			_ => {
				context.exit(WellKnownReason::unhandled_signal(signal).into()).await;
				unreachable!()
			},
		}
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		mut self,
		context: &mut Ctx,
		_exit_reason: ExitReason,
	) {
		let mut wait_list = vec![];

		self.children.drain().for_each(|child_id| {
			context.system().stop(child_id, ExitReason::shutdown());
			wait_list.push(child_id);
		});

		let _ = futures::future::join_all(
			wait_list.into_iter().map(|child_id| context.system().wait(child_id)),
		)
		.await;
	}
}

impl<WA, S, Rq> DynamicSup<WA, S, Rq> {
	async fn handle_start_child<Ctx>(
		&mut self,
		context: &mut Ctx,
		reply_to: oneshot::Tx<Result<ActorID, StartError>>,
		rq: Rq,
	) -> Result<(), SupError>
	where
		Ctx: Context<Seed = S>,
		S: DynSupSeed<Rq, WA::Seed>,
		WA: Actor<Ctx::System>,
		Rq: Message,
	{
		let start_opts = StartOpts::new().link(context.actor_id());
		let child_seed = self.seed.produce_seed(rq).ok_or(SupError::SeedMissing)?;
		let result = context.start::<WA>(child_seed, start_opts).map_err(StartError::system_error);

		if let Ok(child_id) = result.as_ref() {
			self.children.insert(*child_id);
		}
		reply_to.send(result);

		Ok(())
	}

	async fn handle_stop_child<Ctx>(
		&mut self,
		context: &mut Ctx,
		reply_to: oneshot::Tx<Result<(), StopError>>,
		child_id: ActorID,
	) -> Result<(), SupError>
	where
		Ctx: Context,
	{
		if self.children.remove(&child_id) {
			context.system().stop(child_id, ExitReason::shutdown());
			self.exitting.insert(child_id, reply_to);
		} else {
			reply_to.send(Err(StopError::NotFound));
		}

		Ok(())
	}
}
