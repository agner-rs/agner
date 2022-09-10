use std::time::Duration;

use crate::actor::Actor;
use crate::actor_id::ActorID;
use crate::error::{Error, ExitReason};
use crate::message::Message;
use crate::start_opts::StartOpts;

#[async_trait::async_trait]
pub trait System: Sized + Send + Sync + 'static {
	type Error: Error;

	fn start<A: Actor<Self>>(
		&self,
		seed: A::Seed,
		opts: StartOpts<A::Seed, A::Message>,
	) -> Result<ActorID, Self::Error>;
	fn send<M: Message>(&self, to: ActorID, message: M);
	fn stop(&self, actor: ActorID, exit_reason: ExitReason);

	async fn wait(&self, actor_id: ActorID) -> ExitReason;
	async fn shutdown(&self, timeout: Duration);
}
