use crate::error::ExitReason;
use crate::{Actor, ActorID, Message, Seed, StartOpts, System, TaskManager};

#[async_trait::async_trait]
pub trait Context: Send + Sync {
	type Actor: Actor<Self::System>;
	type System: System;
	type Seed: Seed;
	type TaskManager: TaskManager<<Self::Actor as Actor<Self::System>>::Message>;

	fn actor_id(&self) -> ActorID;

	fn seed(&self) -> &Self::Seed;
	fn seed_mut(&mut self) -> &mut Self::Seed;

	fn start<A: Actor<Self::System>>(
		&mut self,
		seed: A::Seed,
		opts: StartOpts<A::Seed, A::Message>,
	) -> Result<ActorID, <Self::System as System>::Error>;

	fn system(&self) -> &Self::System;
	fn tasks(&mut self) -> &mut Self::TaskManager;

	fn send<M: Message>(&mut self, to: ActorID, message: M);

	fn link(&mut self, to: ActorID);

	fn order_exit(&mut self, reason: ExitReason);
	async fn exit(&mut self, reason: ExitReason) -> std::convert::Infallible {
		self.order_exit(reason);
		std::future::pending().await
	}
}
