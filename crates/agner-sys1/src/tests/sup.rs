use agner_actor::error::*;
use agner_actor::{
	oneshot, Actor, ActorID, Context, Message, Seed, SeedMut, Signal, StartOpts, System, UniqueSeed,
};

#[derive(Debug)]
pub struct Sup<A, S, M> {
	_pd: std::marker::PhantomData<(A, S)>,
	child: ActorID,
	seed_heir_rx: oneshot::Rx<S>,
	inbox_heir_rx: oneshot::Rx<Vec<M>>,
}

#[derive(Debug)]
pub enum SupRq<M> {
	Pass(M),
	ChildID(oneshot::Tx<ActorID>),
}

#[async_trait::async_trait]
impl<Sys, A, S, M> Actor<Sys> for Sup<A, S, M>
where
	Sys: System,
	A: Actor<Sys, Seed = S, Message = M>,
	S: Seed,
	M: Message,
{
	type Seed = UniqueSeed<S>;
	type Message = SupRq<<A as Actor<Sys>>::Message>;

	async fn init<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		let seed = context.seed_mut().take_value().expect("Empty seed provided");
		let (seed_heir_tx, seed_heir_rx) = oneshot::channel();
		let (inbox_heir_tx, inbox_heir_rx) = oneshot::channel();
		let start_opts = StartOpts::new()
			.link(context.actor_id())
			.seed_heir(seed_heir_tx)
			.inbox_heir(inbox_heir_tx);
		let child = context.start::<A>(seed, start_opts).map_err(ExitReason::error_actor)?;

		Ok(Self { _pd: Default::default(), child, seed_heir_rx, inbox_heir_rx })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		match message {
			SupRq::Pass(message) => context.send(self.child, message),
			SupRq::ChildID(reply_to) => reply_to.send(self.child),
		}
		Ok(())
	}

	async fn handle_signal<Ctx: Context<System = Sys>>(
		&mut self,
		context: &mut Ctx,
		signal: Signal,
	) -> Result<(), ExitReason> {
		match signal {
			Signal::Exit(from, reason) if from == self.child => {
				log::info!("child {:?} down: {:?}", from, reason);

				let (seed_heir_tx, seed_heir_rx) = oneshot::channel();
				let seed_heir_rx = std::mem::replace(&mut self.seed_heir_rx, seed_heir_rx);
				let seed = seed_heir_rx.await.expect("Seed did not return from the dead child");

				let (inbox_heir_tx, inbox_heir_rx) = oneshot::channel();
				let inbox_heir_rx = std::mem::replace(&mut self.inbox_heir_rx, inbox_heir_rx);
				let inbox = inbox_heir_rx.await.expect("Inbox did not return from the dead child");

				let start_opts = StartOpts::new()
					.link(context.actor_id())
					.seed_heir(seed_heir_tx)
					.inbox_heir(inbox_heir_tx)
					.inbox(inbox);
				let child =
					context.start::<A>(seed, start_opts).map_err(ExitReason::error_actor)?;
				self.child = child;
				log::info!("child restarted: {:?}", child);

				Ok(())
			},
			unhandled => {
				context.exit(WellKnownReason::unhandled_signal(unhandled).into()).await;
				unreachable!()
			},
		}
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		log::trace!("{} Sup::terminated [reason: {}]", context.actor_id(), exit_reason);
	}
}
