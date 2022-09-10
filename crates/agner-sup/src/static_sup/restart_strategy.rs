use agner_actor::error::ExitReason;

mod give_up;
pub use give_up::GiveUp;

mod one_for_one;
pub use one_for_one::OneForOne;

#[derive(Debug, Clone)]
pub enum Action {
	Start { child: usize },
	Stop { child: usize, reason: ExitReason },
	Exit { reason: ExitReason },
}

pub trait RestartStrategy: Clone + Send + Sync + 'static {
	type State: Send + Sync + 'static;

	fn init(&self, children_count: usize) -> Self::State;
	fn child_up(&self, state: &mut Self::State, idx: usize);
	fn child_down(&self, state: &mut Self::State, idx: usize, reason: ExitReason);
	fn terminate(&self, state: &mut Self::State);

	fn next_action(&self, state: &mut Self::State) -> Option<Action>;
}
