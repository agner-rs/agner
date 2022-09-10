use agner_actor::error::ExitReason;
use agner_actor::{oneshot, NonEmptySeed, SharedSeed};

use agner_actor::{Actor, ActorID, Context, StartOpts, System};

#[derive(Debug)]
pub struct RingActor {
	id: usize,
	counter: usize,
	next: Option<ActorID>,
}

#[derive(Debug)]
pub struct RingArgs {
	pub id: usize,
}

#[derive(Debug)]
pub struct EchoRequest {
	pub rq_id: usize,
	pub reply_to: oneshot::Tx<EchoResponse>,
}

#[derive(Debug)]
pub struct EchoResponse {
	pub actor: usize,
	pub rq_id: usize,
	pub rs_id: usize,
}

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for RingActor {
	type Seed = SharedSeed<RingArgs>;
	type Message = EchoRequest;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		let id = context.seed().value().id;

		let next = if id == 0 {
			None
		} else {
			Some(
				context
					.start::<Self>(
						RingArgs { id: id - 1 }.into(),
						StartOpts::new().link(context.actor_id()),
					)
					.expect("Failed to start actor"),
			)
		};

		Ok(Self { id, counter: 0, next })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		request: Self::Message,
	) -> Result<(), ExitReason> {
		self.counter += 1;
		if let Some(next) = self.next {
			context.send(next, request);
		} else {
			let reply_to = request.reply_to;
			let response =
				EchoResponse { actor: self.id, rq_id: request.rq_id, rs_id: self.counter };
			let _ = reply_to.send(response);
		}
		Ok(())
	}
}
