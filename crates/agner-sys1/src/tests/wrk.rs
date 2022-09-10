use agner_actor::error::{ExitError, ExitReason};
use agner_actor::{oneshot, Actor, Context, Seed, SeedMut, System, UniqueSeed};

#[derive(Debug)]
pub struct Wrk {
	incarnation: usize,
	rq_counter: usize,
}

#[derive(Debug)]
pub enum WrkRq {
	Exit,
	Rq(oneshot::Tx<(usize, usize)>),
}

#[derive(Debug, thiserror::Error)]
#[error("Exit Requested")]
pub struct ExitRequested;

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for Wrk {
	type Seed = UniqueSeed<usize>;
	type Message = WrkRq;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		log::trace!("{} Wrk::init [seed: {:?}]", context.actor_id(), context.seed());
		let incarnation = *context.seed().value_opt().expect("Seed missing");
		*context.seed_mut().value_mut().expect("Seed missing") += 1;
		Ok(Self { incarnation, rq_counter: 0 })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		self.rq_counter += 1;

		log::trace!(
			"{} Wrk::handle_message [incarnation: {:?}, message: {:?}]",
			context.actor_id(),
			self.incarnation,
			message,
		);

		match message {
			WrkRq::Rq(reply_to) => reply_to.send((self.incarnation, self.rq_counter)),
			WrkRq::Exit => {
				context.exit(ExitError::actor(ExitRequested).into()).await;
				unreachable!()
			},
		}

		Ok(())
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		log::trace!("{} Wrk::terminated [reason: {}]", context.actor_id(), exit_reason);
	}
}
