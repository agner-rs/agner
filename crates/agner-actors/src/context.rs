use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::actor_runner::call_msg::CallMsg;
use crate::actor_runner::pipe::{PipeRx, PipeTx};
use crate::system::{System, SystemOpt};
use crate::{ExitReason, Never};

/// Actor's API to itself
#[derive(Debug)]
pub struct Context<M> {
	actor_id: ActorID,
	system: SystemOpt,
	messages: PipeRx<M>,
	signals: PipeRx<Signal>,
	calls: PipeTx<CallMsg>,
}

#[derive(Debug)]
pub enum Event<M> {
	Message(M),
	Signal(Signal),
}

#[derive(Debug)]
pub enum Signal {
	Exited(ActorID, Arc<ExitReason>),
}

impl<M> Context<M> {
	/// Get current actor's [`ActorID`]
	pub fn actor_id(&self) -> ActorID {
		self.actor_id
	}

	/// Get the [`System`] this actor is running in.
	pub fn system(&self) -> System {
		self.system.rc_upgrade().expect("System gone")
	}

	pub async fn next_event(&mut self) -> Event<M>
	where
		M: Unpin,
	{
		tokio::select! {
			biased;

			signal = self.signals.recv() =>
				Event::Signal(signal),
			message = self.messages.recv() =>
				Event::Message(message),
		}
	}
}

impl<M> Context<M> {
	async fn backend_call(&mut self, call: CallMsg) {
		self.calls.send(call).await.expect("Failed to send CallMsg");
	}

	pub async fn exit(&mut self, exit_reason: ExitReason) -> Never {
		self.backend_call(CallMsg::Exit(exit_reason)).await;
		std::future::pending().await
	}

	pub async fn link(&mut self, to: ActorID) {
		self.backend_call(CallMsg::Link(to)).await;
	}
	pub async fn unlink(&mut self, from: ActorID) {
		self.backend_call(CallMsg::Unlink(from)).await;
	}
}

impl<M> Context<M> {
	/// Create a new instance of [`Context`]
	pub(crate) fn new(
		actor_id: ActorID,
		system: SystemOpt,
		inbox: PipeRx<M>,
		signals: PipeRx<Signal>,
		calls: PipeTx<CallMsg>,
	) -> Self {
		Self { actor_id, system, messages: inbox, signals, calls }
	}
}
