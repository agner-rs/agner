use std::marker::PhantomData;

use agner_actors::{Actor, ActorID, BoxError, Context, SpawnOpts};
use tokio::sync::oneshot;

pub type SpawnError = BoxError;

pub enum Message<I> {
	StartChild(I, oneshot::Sender<Result<ActorID, SpawnError>>),
}

pub struct ChildSpec<B, A, I, O, M> {
	behaviour: B,
	arg_factory: A,
	_pd: PhantomData<(I, O, M)>,
}

pub fn child_spec<B, A, I, O, M>(behaviour: B, arg_factory: A) -> ChildSpec<B, A, I, O, M>
where
	A: FnMut(I) -> O,
	for<'a> B: Actor<'a, O, M>,
	B: Clone,
{
	ChildSpec { behaviour, arg_factory, _pd: Default::default() }
}

pub async fn dynamic_sup<B, A, I, O, M>(
	context: &mut Context<Message<I>>,
	child_spec: ChildSpec<B, A, I, O, M>,
) {
}

#[derive(Debug)]
struct ChildSpecImpl<B, A> {
	behaviour: B,
	arg_factory: A,
}

impl<B, A, I, O, M> ChildSpec<B, A, I, O, M>
where
	A: FnMut(I) -> O,
	for<'a> B: Actor<'a, O, M>,
	M: Send + Sync + Unpin + 'static,
	O: Send + Sync + 'static,
	B: Clone + Send + Sync + 'static,
{
	async fn spawn(
		&mut self,
		context: &mut Context<Message<I>>,
		arg: I,
	) -> Result<ActorID, SpawnError> {
		let arg = (self.arg_factory)(arg);
		let spawn_opts = SpawnOpts::new();
		context.system().spawn(self.behaviour.to_owned(), arg, spawn_opts).await
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn ergonomics() {
		use agner_actors::Context;

		pub enum Request {}
		async fn child_actor(
			context: &mut Context<Request>,
			(group_name, worker_id): (&'static str, usize),
		) {
		}

		let child_spec =
			super::child_spec(child_actor, |worker_id: usize| ("group-name", worker_id));
	}
}
