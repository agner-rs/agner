use std::collections::HashMap;
use std::future::Future;

use agner_actor::error::ExitReason;
use agner_actor::{Actor, ActorID, Context, NonEmptySeed, SharedSeed, StartOpts, System};

use agner_actor::oneshot;

pub mod api {
	use agner_actor::{ActorID, System};
	use rand::RngCore;

	use super::TreeRequest;

	pub async fn ping(system: &impl System, actor_id: ActorID) -> bool {
		let id = rand::thread_rng().next_u64();
		let (rq, rx) = TreeRequest::ping(id);
		system.send(actor_id, rq);

		rx.await.map(|rs| rs == id).unwrap_or(false)
	}

	pub async fn lookup_child(
		system: &impl System,
		mut actor_id: ActorID,
		path: impl IntoIterator<Item = impl Into<String>>,
	) -> Option<ActorID> {
		log::debug!("lookup_child [of: {}]", actor_id);
		let mut path = path.into_iter();

		while let Some(p) = path.next() {
			let p = p.into();
			log::debug!("  - {:?}", p);
			let (request, response) = TreeRequest::get_child_by_name(p);
			system.send(actor_id, request);
			actor_id = response.await?;

			log::debug!("    -> {}", actor_id);
		}
		Some(actor_id)
	}
}

#[derive(Debug)]
pub struct TreeActor {
	name: String,
	children: HashMap<String, ActorID>,
}

#[derive(Debug, Clone)]
pub struct TreeArgs {
	name: String,
	link: bool,
	children: Vec<Self>,
}

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
	#[error("Failed to start a child")]
	FailedToStartChild,
}

impl TreeArgs {
	pub fn new(name: impl Into<String>) -> Self {
		Self { name: name.into(), link: true, children: Default::default() }
	}
	pub fn link(self, link: bool) -> Self {
		Self { link, ..self }
	}
	pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
		self.children.extend(children);
		self
	}
}

#[derive(Debug)]
pub enum TreeRequest {
	Ping(u64, oneshot::Tx<u64>),
	GetChildByName(String, oneshot::Tx<Option<ActorID>>),
}
impl TreeRequest {
	pub fn ping(id: u64) -> (Self, impl Future<Output = Option<u64>>) {
		let (tx, rx) = oneshot::channel();
		let request = Self::Ping(id, tx);
		(request, rx)
	}
	pub fn get_child_by_name(
		name: impl Into<String>,
	) -> (Self, impl Future<Output = Option<ActorID>>) {
		let (tx, rx) = oneshot::channel();
		let request = Self::GetChildByName(name.into(), tx);
		(request, async move { rx.await.flatten() })
	}
}

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for TreeActor {
	type Seed = SharedSeed<TreeArgs>;
	type Message = TreeRequest;

	async fn init<Ctx: Context<Seed = Self::Seed>>(context: &mut Ctx) -> Result<Self, ExitReason> {
		log::trace!("[{} — {:?}] init", context.actor_id(), context.seed().value().name);

		let name = context.seed().value().name.to_owned();
		let mut children = HashMap::new();

		for child_spec in context.seed().value().children.to_owned() {
			let child_name = child_spec.name.to_owned();
			let should_link = child_spec.link;

			let start_opts = StartOpts::new();
			let start_opts =
				if should_link { start_opts.link(context.actor_id()) } else { start_opts };
			let child_actor_id = context
				.start::<Self>(child_spec.into(), start_opts)
				.map_err(|system_err| {
					log::error!("faield to start child: {}", system_err);
					TreeError::FailedToStartChild
				})
				.map_err(ExitReason::error_actor)?;

			children.insert(child_name, child_actor_id);
		}

		Ok(Self { name, children })
	}

	async fn handle_message<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		log::trace!("[{} - {:?}] handle_message {:?}", context.actor_id(), self.name, message);

		match message {
			TreeRequest::Ping(id, reply_to) => {
				let _ = reply_to.send(id);
			},
			TreeRequest::GetChildByName(name, reply_to) => {
				let response = self.children.get(&name).copied();
				let _ = reply_to.send(response);
			},
		}

		Ok(())
	}
}
