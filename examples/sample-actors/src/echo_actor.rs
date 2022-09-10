use agner_actor::error::ExitReason;
use agner_actor::{oneshot, Actor, Context, NonEmptySeed, SharedSeed, System};

#[derive(Debug)]
pub struct EchoActor {
	id: usize,
	counter: usize,
}

#[derive(Debug)]
pub struct EchoArgs {
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
impl<Sys: System> Actor<Sys> for EchoActor {
	type Seed = SharedSeed<EchoArgs>;
	type Message = EchoRequest;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		Ok(Self { id: context.seed().value().id, counter: 0 })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		_context: &mut Ctx,
		request: Self::Message,
	) -> Result<(), ExitReason> {
		self.counter += 1;
		let reply_to = request.reply_to;
		let response = EchoResponse { actor: self.id, rq_id: request.rq_id, rs_id: self.counter };
		let _ = reply_to.send(response);
		Ok(())
	}
}
