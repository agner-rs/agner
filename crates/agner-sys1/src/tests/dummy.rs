use std::fmt;

use agner_actor::error::ExitReason;
use agner_actor::{Actor, Context, Message, NonEmptySeed, SharedSeed, System};

#[derive(Debug)]
pub struct DummyActor<S, M> {
	_pd: std::marker::PhantomData<(S, M)>,
}

#[async_trait::async_trait]
impl<Sys: System, S: fmt::Debug + Send + Sync + 'static, M: Message + fmt::Debug> Actor<Sys>
	for DummyActor<S, M>
{
	type Seed = SharedSeed<S>;
	type Message = M;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		log::trace!(
			"{} SimplestActor::init [seed: {:?}]",
			context.actor_id(),
			context.seed().value()
		);
		Ok(Self { _pd: Default::default() })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		log::trace!(
			"{} SimplestActor::handle_message [message: {:?}]",
			context.actor_id(),
			message
		);

		Ok(())
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		log::trace!("{} SimplestActor::terminated [reason: {}]", context.actor_id(), exit_reason);
	}
}
