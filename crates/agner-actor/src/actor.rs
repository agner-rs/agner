use crate::context::Context;
use crate::error::{ExitReason, WellKnownReason};
use crate::message::Message;
use crate::signal::Signal;
use crate::{Seed, System};

#[async_trait::async_trait]
pub trait Actor<Sys: System>: Sized + Send + Sync + 'static {
	type Seed: Seed;
	type Message: Message;

	async fn init<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason>;

	async fn handle_message<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		let _ = (context, message);
		Ok(())
	}

	async fn handle_signal<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		signal: Signal,
	) -> Result<(), ExitReason> {
		context.exit(WellKnownReason::unhandled_signal(signal).into()).await;
		unreachable!()
	}

	async fn terminated<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		let _ = (context, exit_reason);
	}
}
