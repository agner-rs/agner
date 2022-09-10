use crate::{oneshot, ActorID};

#[derive(Debug)]
pub struct StartOpts<Seed, Msg> {
	pub link_with: Vec<ActorID>,
	pub seed_heir: Option<oneshot::Tx<Seed>>,
	pub inbox_heir: Option<oneshot::Tx<Vec<Msg>>>,
	pub inbox: Vec<Msg>,
}

impl<Seed, Msg> Default for StartOpts<Seed, Msg> {
	fn default() -> Self {
		Self {
			link_with: Default::default(),
			seed_heir: None,
			inbox_heir: None,
			inbox: Default::default(),
		}
	}
}

impl<Seed, Msg> StartOpts<Seed, Msg> {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn link(mut self, with: ActorID) -> Self {
		self.link_with.push(with);
		self
	}

	pub fn seed_heir(mut self, heir: oneshot::Tx<Seed>) -> Self {
		self.seed_heir = Some(heir);
		self
	}

	pub fn inbox_heir(mut self, heir: oneshot::Tx<Vec<Msg>>) -> Self {
		self.inbox_heir = Some(heir);
		self
	}

	pub fn inbox(mut self, messages: impl IntoIterator<Item = Msg>) -> Self {
		self.inbox.extend(messages);
		self
	}
}
