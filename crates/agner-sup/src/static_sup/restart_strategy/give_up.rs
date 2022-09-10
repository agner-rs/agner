use super::*;

#[derive(Debug, Clone)]
pub struct GiveUp;

impl RestartStrategy for GiveUp {
	type State = bool;

	fn init(&self, _children_count: usize) -> Self::State {
		false
	}
	fn child_up(&self, _state: &mut Self::State, _idx: usize) {}
	fn child_down(&self, state: &mut Self::State, _idx: usize, _reason: ExitReason) {
		*state = true
	}
	fn terminate(&self, _state: &mut Self::State) {}
	fn next_action(&self, state: &mut Self::State) -> Option<Action> {
		if *state {
			Some(Action::Exit { reason: ExitReason::shutdown() })
		} else {
			None
		}
	}
}
