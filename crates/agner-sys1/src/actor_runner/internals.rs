use std::future::Future;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use agner_actor::error::ExitReason;
use agner_actor::{oneshot, ActorID, Signal, TaskID};
use futures::stream::FuturesUnordered;
use futures::StreamExt;

const DEFAULT_INBOX_MAX_LEN: usize = 1024;

pub(super) struct Internals<M> {
	actor_id: ActorID,
	inbox: VecDeque<M>,
	inbox_max_len: usize,

	signals: VecDeque<Signal>,
	linked: HashSet<ActorID>,
	waiting: Vec<oneshot::Tx<ExitReason>>,

	task_killswitches: HashMap<TaskID, oneshot::Tx<()>>,
	tasks: FuturesUnordered<TaskFuture>,

	exit_opt: Option<ExitReason>,
}

impl<M> Internals<M> {
	pub fn create(actor_id: ActorID, inbox: impl IntoIterator<Item = M>) -> Self {
		Self {
			actor_id,
			inbox_max_len: DEFAULT_INBOX_MAX_LEN,
			inbox: inbox.into_iter().collect(),
			linked: Default::default(),
			waiting: Default::default(),
			signals: Default::default(),
			task_killswitches: Default::default(),
			tasks: Default::default(),
			exit_opt: None,
		}
	}

	pub fn inbox_enq(&mut self, message: M) -> Result<(), M> {
		if self.inbox.len() >= self.inbox_max_len {
			Err(message)
		} else {
			self.inbox.push_back(message);
			Ok(())
		}
	}

	pub fn inbox_deq(&mut self) -> Option<M> {
		self.inbox.pop_front()
	}

	pub fn inbox_drain<'a>(&'a mut self) -> impl Iterator<Item = M> + 'a {
		self.inbox.drain(..)
	}

	pub fn signal_enq(&mut self, signal: Signal) {
		self.signals.push_back(signal);
	}
	pub fn signal_deq(&mut self) -> Option<Signal> {
		self.signals.pop_front()
	}

	pub fn request_exit(&mut self, exit_reason: ExitReason) {
		if self.exit_opt.is_some() {
			log::trace!("[{}] Requested exit more than once", self.actor_id);
		} else {
			self.exit_opt = Some(exit_reason);
		}
	}

	pub fn tasks_add(
		&mut self,
		tasks: impl IntoIterator<
			Item = (TaskID, Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>),
		>,
	) {
		for (task_id, task_future) in tasks {
			let (kill_switch, task_cancelled) = oneshot::channel();
			self.task_killswitches.insert(task_id, kill_switch);
			self.tasks
				.push(TaskFuture { task_id, task_cancelled, task_running: task_future });
		}
	}
	pub fn tasks_rm(&mut self, tasks: impl IntoIterator<Item = TaskID>) {
		for task_id in tasks {
			let _ = self.task_killswitches.remove(&task_id);
		}
	}

	pub async fn tasks_poll(&mut self) -> () {
		if self.tasks.is_empty() {
			std::future::pending().await
		} else {
			let (task_id, _task_completed) = self.tasks.next().await.expect("self.tasks is empty?");
			self.task_killswitches.remove(&task_id);
		}
	}

	pub fn exit_requested(&self) -> Option<&ExitReason> {
		self.exit_opt.as_ref()
	}

	pub fn link_add(&mut self, id: ActorID) {
		self.linked.insert(id);
	}
	pub fn link_rm(&mut self, id: ActorID) {
		self.linked.remove(&id);
	}
	pub fn linked<'a>(&'a self) -> impl Iterator<Item = ActorID> + 'a {
		self.linked.iter().copied()
	}
	pub fn waiting_drain<'a>(&'a mut self) -> impl Iterator<Item = oneshot::Tx<ExitReason>> + 'a {
		self.waiting.drain(..)
	}
	pub fn waiting_add(&mut self, waiting: oneshot::Tx<ExitReason>) {
		self.waiting.push(waiting);
	}
}

impl<M> fmt::Debug for Internals<M> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Internals")
			.field("actor-id", &self.actor_id)
			.field("inbox.len", &self.inbox.len())
			.field("inbox-max-len", &self.inbox_max_len)
			.field("signals", &self.signals)
			.field("linked", &self.linked)
			.field("waiting.len", &self.waiting.len())
			.field("exit-opt", &self.exit_opt)
			.finish()
	}
}

#[pin_project::pin_project]
struct TaskFuture {
	task_id: TaskID,

	#[pin]
	task_cancelled: oneshot::Rx<()>,

	task_running: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
}
impl Future for TaskFuture {
	type Output = (TaskID, bool);
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		let task_id = *this.task_id;

		match this.task_cancelled.poll(cx) {
			Poll::Pending => {
				let inner = this.task_running.as_mut();
				let () = futures::ready!(inner.poll(cx));
				Poll::Ready((task_id, true))
			},
			Poll::Ready(_) => Poll::Ready((task_id, false)),
		}
	}
}
