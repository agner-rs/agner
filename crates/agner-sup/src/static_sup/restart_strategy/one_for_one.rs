use std::collections::VecDeque;

use agner_actor::error::ExitReason;

use super::{Action, RestartStrategy};

#[derive(Debug, Clone)]
pub struct OneForOne {}

impl OneForOne {
	pub fn new() -> Self {
		Self {}
	}
}

#[derive(Debug)]
pub struct OneForOneState {}

pub struct State {
	children_count: usize,
	actions: VecDeque<Action>,
}

impl RestartStrategy for OneForOne {
	type State = State;

	fn init(&self, children_count: usize) -> Self::State {
		State { children_count, actions: Default::default() }
	}
	fn child_up(&self, _state: &mut Self::State, _idx: usize) {}
	fn child_down(&self, state: &mut Self::State, idx: usize, _reason: ExitReason) {
		assert!(idx < state.children_count);
		state.actions.push_back(Action::Start { child: idx });
	}
	fn terminate(&self, _state: &mut Self::State) {}
	fn next_action(&self, state: &mut Self::State) -> Option<Action> {
		state.actions.pop_front()
	}
}
