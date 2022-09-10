use agner_actor::ActorID;

#[derive(Debug)]
pub struct ChildState {
	pub run_state: RunState,
}

#[derive(Debug, Clone, Copy)]
pub enum RunState {
	Running(ActorID),
	Stopped,
}

impl RunState {
	pub fn actor_id(&self) -> Option<ActorID> {
		if let Self::Running(actor_id) = *self {
			Some(actor_id)
		} else {
			None
		}
	}
}

impl Default for ChildState {
	fn default() -> Self {
		Self { run_state: RunState::Stopped }
	}
}
