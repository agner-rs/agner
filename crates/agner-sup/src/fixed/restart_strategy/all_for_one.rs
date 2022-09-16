use std::collections::VecDeque;
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
    ch_ids: Box<[ActorID]>,
    ch_states: Box<[ChState]>,
    failures: Box<[FrequencyStats]>,
    pending: VecDeque<Action>,
}

#[derive(Debug, Clone, Copy)]
enum ChState {
    Down,
    Up,
    ShuttingDown,
}

impl RestartStrategy for AllForOne {
    type Decider = AllForOneDecider;

    fn new_decider(&self, sup_id: ActorID, children: &[ActorID]) -> Self::Decider {
        let failures = children.iter().map(|_| self.frequency_policy.new_stats()).collect();
        let ch_states = children.iter().map(|_| ChState::Up).collect();
        let ch_ids = children.into();
        AllForOneDecider { sup_id, ch_ids, ch_states, failures, pending: Default::default() }
    }
}

impl Decider for AllForOneDecider {
    fn next_action(&mut self) -> Option<super::Action> {
        self.pending.pop_front()
    }
    fn child_up(&mut self, _at: Instant, child_idx: usize, actor_id: ActorID) {
        assert!(matches!(self.ch_states[child_idx], ChState::Down));
        self.ch_ids[child_idx] = actor_id;
        self.ch_states[child_idx] = ChState::Up;
    }
    fn child_dn(&mut self, at: Instant, actor_id: ActorID, exit_reason: ExitReason) {
        let idx_opt = self
            .ch_ids
            .iter()
            .enumerate()
            .find(|&(_, &id)| id == actor_id)
            .map(|(idx, _)| idx);

        if let Some(idx) = idx_opt {
            if matches!(self.ch_states[idx], ChState::ShuttingDown) {
                self.ch_states[idx] = ChState::Down;
            } else if self.failures[idx].report(at) {
                self.initiate_shutdown(exit_reason)
            } else {
                self.initiate_restart(exit_reason)
            }
        } else {
            log::info!(
                "Unknown linked actor exited. Initiating shutdown. [reason: {}]",
                exit_reason.pp()
            );

            self.initiate_shutdown(ExitReason::Exited(actor_id, exit_reason.into()))
        }
    }
}

impl AllForOneDecider {
    fn initiate_restart(&mut self, cause: ExitReason) {
        let arc_cause = Arc::new(cause);

        self.pending.clear();
        self.pending.extend(
            (0..self.ch_ids.len())
                .rev()
                .filter_map(|idx| {
                    if !matches!(self.ch_states[idx], ChState::Down) {
                        self.ch_states[idx] = ChState::ShuttingDown;
                        Some(Action::Stop(
                            self.ch_ids[idx],
                            ExitReason::Shutdown(Some(arc_cause.to_owned())),
                        ))
                    } else {
                        None
                    }
                })
                .chain((0..self.ch_ids.len()).map(Action::Start)),
        );
    }

    fn initiate_shutdown(&mut self, exit_reason: ExitReason) {
        let arc_exit_reason = Arc::new(exit_reason.to_owned());

        self.pending.clear();
        self.pending.extend(
            (0..self.ch_ids.len())
                .rev()
                .filter_map(|idx| {
                    if !matches!(self.ch_states[idx], ChState::Down) {
                        self.ch_states[idx] = ChState::ShuttingDown;
                        Some(Action::Stop(
                            self.ch_ids[idx],
                            ExitReason::Shutdown(Some(arc_exit_reason.to_owned())),
                        ))
                    } else {
                        None
                    }
                })
                .chain([Action::Exit(exit_reason)]),
        );
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
