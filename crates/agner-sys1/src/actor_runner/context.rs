use std::future::Future;
use std::pin::Pin;

use agner_actor::task::Done;
use tokio::sync::mpsc;

use agner_actor::error::{ExitReason, WellKnownReason};
use agner_actor::{Actor, ActorID, Message, StartOpts, System, TaskID};

use crate::sys_msg::SysMsg;
use crate::system::SystemOne;

pub(super) struct Context<'s, A: Actor<SystemOne>> {
	_pd: std::marker::PhantomData<A>,
	system: SystemOne,
	seed: &'s mut A::Seed,
	actor_id: ActorID,
	tx_sys: mpsc::UnboundedSender<SysMsg>,
	next_task_id: usize,
	tasks_to_start: Vec<(TaskID, Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>)>,
	tasks_to_stop: Vec<TaskID>,
}

impl<'s, A: Actor<SystemOne>> Context<'s, A> {
	pub fn create(
		seed: &'s mut A::Seed,
		system: SystemOne,
		actor_id: ActorID,
		tx_sys: mpsc::UnboundedSender<SysMsg>,
	) -> Self {
		Self {
			_pd: Default::default(),
			seed,
			system,
			actor_id,
			tx_sys,
			next_task_id: 0,
			tasks_to_start: Default::default(),
			tasks_to_stop: Default::default(),
		}
	}

	pub(crate) fn tasks_to_start_drain<'a>(
		&'a mut self,
	) -> impl Iterator<Item = (TaskID, Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>)> + 'a
	{
		self.tasks_to_start.drain(..)
	}
	pub(crate) fn tasks_to_stop_drain<'a>(&'a mut self) -> impl Iterator<Item = TaskID> + 'a {
		self.tasks_to_stop.drain(..)
	}
}

impl<'s, A: Actor<SystemOne>> agner_actor::Context for Context<'s, A> {
	type Actor = A;
	type System = SystemOne;
	type Seed = A::Seed;
	type TaskManager = Self;

	fn actor_id(&self) -> ActorID {
		self.actor_id
	}

	fn seed(&self) -> &Self::Seed {
		self.seed
	}
	fn seed_mut(&mut self) -> &mut Self::Seed {
		self.seed
	}

	fn start<S: Actor<SystemOne>>(
		&mut self,
		seed: S::Seed,
		opts: StartOpts<S::Seed, S::Message>,
	) -> Result<ActorID, <Self::System as System>::Error> {
		self.system.start::<S>(seed, opts)
	}

	fn system(&self) -> &Self::System {
		&self.system
	}
	fn tasks(&mut self) -> &mut Self::TaskManager {
		self
	}

	fn send<M: Message>(&mut self, to: ActorID, message: M) {
		self.system.send(to, message);
	}
	fn order_exit(&mut self, reason: ExitReason) {
		self.system.stop(self.actor_id, reason);
	}
	fn link(&mut self, to: ActorID) {
		if !self.system.send_sys(to, SysMsg::Link(self.actor_id)) {
			let _ = self.tx_sys.send(SysMsg::Exit(to, WellKnownReason::NoActor(to).into()));
		}
	}
}

impl<'a, A> agner_actor::TaskManager<A::Message> for Context<'a, A>
where
	A: Actor<SystemOne>,
{
	fn start<F>(&mut self, future: F) -> TaskID
	where
		F: Future + Send + Sync + 'static,
		A::Message: From<Done<F::Output>>,
	{
		let task_id: TaskID = {
			let task_id = self.next_task_id;
			self.next_task_id += 1;
			task_id.into()
		};

		let system = self.system.to_owned();
		let actor_id = self.actor_id;

		let task_dispatched =
			make_future_dispatch_task::<A::Message, F>(system, actor_id, task_id, future);
		let boxed = Box::pin(task_dispatched);
		self.tasks_to_start.push((task_id, boxed));

		task_id
	}

	fn stop(&mut self, task_id: TaskID) {
		self.tasks_to_stop.push(task_id);
	}
}

fn make_future_dispatch_task<M, F>(
	system: SystemOne,
	actor_id: ActorID,
	task_id: TaskID,
	future: F,
) -> impl Future<Output = ()> + Send + Sync + 'static
where
	F: Future + Send + Sync + 'static,
	M: From<Done<F::Output>> + Send + Sync + 'static,
{
	async move {
		let value = future.await;
		let message: M = Done { task_id, value }.into();
		system.send(actor_id, message);
	}
}
