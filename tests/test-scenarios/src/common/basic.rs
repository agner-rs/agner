use std::future::Future;
use std::time::Duration;

use agner_actor::error::ExitReason;
use agner_actor::{oneshot, NonEmptySeed, SharedSeed};

use agner_actor::{Actor, Context, System};

#[derive(Debug)]
pub struct BasicActor {
	id: usize,
	counter: usize,
}

#[derive(Debug)]
pub struct BasicArgs {
	pub id: usize,
}
impl From<usize> for BasicArgs {
	fn from(id: usize) -> Self {
		Self { id }
	}
}

#[derive(Debug)]
pub struct BasicRequest {
	pub rq_id: usize,
	pub delay: Duration,
	pub reply_to: oneshot::Tx<BasicResponse>,
}

impl BasicRequest {
	pub fn create(
		rq_id: usize,
		delay: Duration,
	) -> (Self, impl Future<Output = Option<BasicResponse>>) {
		let (tx, response_fut) = oneshot::channel();
		let request = Self { rq_id, delay, reply_to: tx };

		(request, response_fut)
	}
}

#[derive(Debug)]
pub struct BasicResponse {
	pub actor_id: usize,
	pub rq_id: usize,
	pub rs_id: usize,
}

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for BasicActor {
	type Message = BasicRequest;
	type Seed = SharedSeed<BasicArgs>;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		log::trace!(
			"BasicActor::init [actor-id: {}; seed: {:?}]",
			context.actor_id(),
			context.seed()
		);
		Ok(Self { id: context.seed().value().id, counter: 0 })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		request: Self::Message,
	) -> Result<(), ExitReason> {
		self.counter += 1;

		log::trace!(
			"BasicActor::handle_message [actor-id: {}; rq-id: {}; rs-id: {}; delay: {:?}]",
			context.actor_id(),
			request.rq_id,
			self.counter,
			request.delay,
		);

		tokio::time::sleep(request.delay).await;

		let _ = request.reply_to.send(BasicResponse {
			rq_id: request.rq_id,
			rs_id: self.counter,
			actor_id: self.id,
		});

		Ok(())
	}
}
