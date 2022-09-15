use std::fmt;
use std::time::Duration;

mod all_for_one;
mod one_for_one;

use agner_actors::ActorID;
pub use all_for_one::AllForOne;
pub use one_for_one::OneForOne;

#[derive(Debug, Clone, Copy, Default)]
pub struct Frequency {
    pub max_restarts: usize,
    pub within: Duration,
}

pub trait RestartStrategy: fmt::Display {
    type Decider: Decider;

    fn new_decider(&self, children: Box<[ActorID]>) -> Self::Decider;
}

pub trait Decider {}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Escalate,
    Stop(ActorID),
    Start(usize),
}

impl fmt::Display for Frequency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "max-restarts {} within {:?}", self.max_restarts, self.within)
    }
}
