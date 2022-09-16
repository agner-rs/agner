use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;

use agner_actors::{ActorID, ExitReason};

use crate::fixed::restart_strategy::{Action, Decider, FrequencyPolicy, Instant, RestartStrategy};

use super::FrequencyStats;

#[derive(Debug, Clone, Default)]
pub struct AllForOne {
    pub frequency_policy: FrequencyPolicy,
}

#[derive(Debug)]
pub struct AllForOneDecider {
    sup_id: ActorID,
    frequency_policy: FrequencyPolicy,
    children: Box<[ActorID]>,
    failures: Box<[FrequencyStats]>,
    ignored_exits: HashSet<ActorID>,
    pending: VecDeque<Action>,
}

impl RestartStrategy for AllForOne {
    type Decider = AllForOneDecider;

    fn new_decider(&self, sup: ActorID, children: Box<[ActorID]>) -> Self::Decider {
        let failures = children.iter().map(|_| self.frequency_policy.new_stats()).collect();
        AllForOneDecider {
            sup_id: sup,
            frequency_policy: self.frequency_policy,
            children,
            failures,
            ignored_exits: Default::default(),
            pending: Default::default(),
        }
    }
}

impl Decider for AllForOneDecider {
    fn next_action(&mut self) -> Option<super::Action> {
        self.pending.pop_front()
    }
    fn child_up(&mut self, at: Instant, child_idx: usize, actor_id: ActorID) {
        self.children[child_idx] = actor_id;
    }
    fn child_dn(&mut self, at: Instant, actor_id: ActorID, exit_reason: Arc<ExitReason>) {
        if self.ignored_exits.remove(&actor_id) {
            log::trace!(
                "[{}] actor exited as expected {}, reason: {}",
                self,
                actor_id,
                exit_reason.pp()
            );
            return
        } else if let Some(idx) = self
            .children
            .iter()
            .enumerate()
            .find_map(|(idx, &id)| Some(idx).filter(|_| actor_id == id))
        {
            if self.failures[idx].report(at) {
                self.ignored_exits.extend(self.children.iter().copied());

                self.pending.clear();
                self.pending.extend(
                    self.children
                        .iter()
                        .rev()
                        .copied()
                        .filter(|&child_id| child_id != actor_id)
                        .map(|child_id| Action::Stop(child_id, ExitReason::Shutdown(None)))
                        .chain([Action::Exit(ExitReason::Shutdown(Some(exit_reason)))]),
                );
            } else {
                // self.pending.push_back(Action::Start(idx));
                unimplemented!()
            }
        } else {
            log::info!(
                "Unknown linked actor exited. Initiating shutdown. [reason: {}]",
                exit_reason.pp()
            );
            self.ignored_exits.extend(self.children.iter().copied());

            self.pending.clear();
            self.pending.extend(
                self.children
                    .iter()
                    .rev()
                    .copied()
                    .map(|child_id| Action::Stop(child_id, ExitReason::Shutdown(None)))
                    .chain([Action::Exit(ExitReason::Shutdown(Some(exit_reason)))]),
            );
        }
    }
}

impl fmt::Display for AllForOne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "all-for-one: {}", self.frequency_policy)
    }
}

impl fmt::Display for AllForOneDecider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}|all-for-one", self.sup_id)
    }
}
