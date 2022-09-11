use std::fmt;
use std::future::Future;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskID(usize);

#[derive(Debug, Clone)]
pub struct Done<T> {
	pub task_id: TaskID,
	pub value: T,
}

pub trait TaskManager<M> {
	fn start<F>(&mut self, future: F) -> TaskID
	where
		F: Future + Send + Sync + 'static,
		M: From<Done<F::Output>>;

	fn stop(&mut self, task_id: TaskID);
}

impl From<usize> for TaskID {
	fn from(inner: usize) -> Self {
		Self(inner)
	}
}
impl Into<usize> for TaskID {
	fn into(self) -> usize {
		self.0
	}
}

impl fmt::Display for TaskID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Task[{}]", self.0)
	}
}
