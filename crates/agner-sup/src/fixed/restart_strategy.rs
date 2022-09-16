use std::fmt;
use std::time::Duration;

mod all_for_one;
mod one_for_one;

use agner_actors::{ActorID, ExitReason};
pub use all_for_one::AllForOne;
pub use one_for_one::OneForOne;

pub type Instant = std::time::Instant;

#[derive(Debug, Clone, Copy, Default)]
pub struct FrequencyPolicy {
    pub max_restarts: usize,
    pub within: Duration,
}

#[derive(Debug, Clone)]
pub struct FrequencyStats {
    policy: FrequencyPolicy,
    failures: Vec<Instant>,
}

pub trait RestartStrategy: fmt::Display {
    type Decider: Decider;

    fn new_decider(&self, sup_id: ActorID, children: &[ActorID]) -> Self::Decider;
}

pub trait Decider {
    fn next_action(&mut self) -> Option<Action>;
    fn child_up(&mut self, at: Instant, child_idx: usize, actor_id: ActorID);
    fn child_dn(&mut self, at: Instant, actor_id: ActorID, exit_reason: ExitReason);
}

#[derive(Debug, Clone)]
pub enum Action {
    Exit(ExitReason),
    Stop(ActorID, ExitReason),
    Start(usize),
}

impl FrequencyPolicy {
    pub fn new_stats(&self) -> FrequencyStats {
        FrequencyStats { policy: *self, failures: Vec::with_capacity(self.max_restarts + 1) }
    }
}
impl FrequencyStats {
    /// Report a failure.
    /// Returns `true` if the policy is exceeded.
    pub fn report(&mut self, at: Instant) -> bool {
        if let Some(to_replace) =
            self.failures.iter_mut().find(|v| v.elapsed() > self.policy.within)
        {
            *to_replace = at
        } else {
            self.failures.push(at);
        }
        self.failures
            .iter()
            .copied()
            .filter(|at| at.elapsed() < self.policy.within)
            .count() >
            self.policy.max_restarts
    }
}

impl fmt::Display for FrequencyPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "max-restarts {} within {:?}", self.max_restarts, self.within)
    }
}
