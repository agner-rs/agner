use std::error::Error as StdError;
use std::fmt;

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use agner_actors::{ActorID, Exit};
use agner_utils::std_error_pp::StdErrorPP;

use crate::mixed::child_id::ChildID;
use crate::mixed::child_spec::ChildType;
use crate::mixed::restart_intensity::{
    DurationToInstant, ElapsedSince, RestartIntensity, RestartStats,
};
use crate::mixed::restart_strategy::{Action, Decider};

#[derive(Debug, thiserror::Error)]
pub enum DeciderError {
    #[error("Duplicate ID")]
    DuplicateId,

    #[error("Unknown ID")]
    UnknownId,

    #[error("Unexpected child state")]
    UnexpectedChildState,
}

#[derive(Debug, thiserror::Error)]
#[error("Max restart intensity reached [child-id: {:?}]", child_id)]
pub struct MaxRestartIntensityReached<ID: ChildID, E: StdError> {
    child_id: ID,

    #[source]
    last_error: E,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartType {
    One,
    All,
    Rest,
}

#[derive(Debug)]
pub struct CommonDecider<ID, D, I> {
    sup: ActorID,
    sup_state: SupState<ID>,

    ch_infos: Vec<ChInfo<ID>>,
    ch_states: Vec<ChState>,

    expected_exits: HashSet<ActorID>,
    orphans: VecDeque<(ID, ActorID)>,

    restart_type: RestartType,
    restart_intensity: RestartIntensity<D>,
    restart_stats: RestartStats<I>,
}

impl<ID, D, I> CommonDecider<ID, D, I>
where
    D: DurationToInstant<Instant = I>,
    I: ElapsedSince<Elapsed = D>,
{
    pub fn new(
        sup: ActorID,
        restart_type: RestartType,
        restart_intensity: RestartIntensity<D>,
    ) -> Self {
        let restart_stats = restart_intensity.new_stats();
        Self {
            sup,
            sup_state: SupState::Running,
            ch_infos: Default::default(),
            ch_states: Default::default(),

            expected_exits: Default::default(),
            orphans: Default::default(),

            restart_type,
            restart_intensity,
            restart_stats,
        }
    }
}

impl<ID, D, I> Decider<ID, D, I> for CommonDecider<ID, D, I>
where
    ID: ChildID,
    I: ElapsedSince<Elapsed = D> + fmt::Debug + Send + 'static,
    D: DurationToInstant<Instant = I> + fmt::Debug + Send + 'static,
{
    type Error = DeciderError;

    fn add_child(&mut self, id: ID, ch_type: ChildType) -> Result<(), Self::Error> {
        self.ensure_state_integrity();
        if self.idx(id).is_ok() {
            return Err(DeciderError::DuplicateId)
        }

        log::trace!(
            "[{}|sup:{:?}] adding child {:?}/{:?}",
            self.sup,
            self.restart_type,
            id,
            ch_type
        );

        let info = ChInfo { id, ch_type };
        let state = ChState::ToStart;

        self.ch_states.push(state);
        self.ch_infos.push(info);

        self.sup_state = SupState::Starting;

        Ok(())
    }

    fn rm_child(&mut self, id: ID) -> Result<(), Self::Error> {
        self.ensure_state_integrity();

        let idx = self.idx(id)?;

        log::trace!("[{}|sup:{:?}] Removing child {:?}", self.sup, self.restart_type, id);

        let info = self.ch_infos.remove(idx);
        let state = self.ch_states.remove(idx);

        assert_eq!(info.id, id);

        if let ChState::Running(ch_actor) = state {
            self.orphans.push_back((id, ch_actor));
        }

        Ok(())
    }

    fn next_action(
        &mut self,
    ) -> Result<Option<crate::mixed::restart_strategy::Action<ID>>, Self::Error> {
        let action_opt = loop {
            self.ensure_state_integrity();

            if let Some((id, actor)) = self.orphans.pop_front() {
                self.expected_exits.insert(actor);
                break Some(Action::Stop(id))
            }

            match &mut self.sup_state {
                SupState::Running => break None,

                SupState::ShuttingDown(exit) => {
                    if let Some((info, state)) = self.ch_infos.pop().zip(self.ch_states.pop()) {
                        if let ChState::Running(actor) = state {
                            self.expected_exits.insert(actor);
                            break Some(Action::Stop(info.id))
                        }
                    } else {
                        break Some(Action::Shutdown(exit.to_owned()))
                    }
                },
                SupState::Restarting(ids_to_stop) =>
                    if let Some(id) = ids_to_stop.pop_front() {
                        let idx = self.idx(id)?;
                        if let ChState::Running(actor) = self.ch_states[idx] {
                            self.ch_states[idx] = ChState::ToStart;
                            self.expected_exits.insert(actor);
                            break Some(Action::Stop(id))
                        }
                    } else {
                        self.sup_state = SupState::Starting;
                    },
                SupState::Starting => {
                    if let Some(idx) = self.ch_states.iter().enumerate().find_map(|(idx, state)| {
                        Some(idx).filter(|_| matches!(state, ChState::ToStart))
                    }) {
                        break Some(Action::Start(self.ch_infos[idx].id))
                    } else {
                        self.sup_state = SupState::Running;
                    }
                },
            }
        };
        Ok(action_opt)
    }

    fn child_started(&mut self, id: ID, actor_id: ActorID) -> Result<(), Self::Error> {
        self.ensure_state_integrity();

        let idx = self.idx(id)?;
        if !matches!(self.ch_states[idx], ChState::ToStart) {
            return Err(DeciderError::UnexpectedChildState)
        }

        log::trace!(
            "[{}|sup:{:?}] child started {:?} -> {}",
            self.sup,
            self.restart_type,
            id,
            actor_id
        );
        self.ch_states[idx] = ChState::Running(actor_id);

        Ok(())
    }
    fn exit_signal(
        &mut self,
        actor_id: ActorID,
        exit: agner_actors::Exit,
        at: I,
    ) -> Result<(), Self::Error> {
        if actor_id == self.sup {
            log::trace!(
                "[{}|sup:{:?}] sup received exit signal: [at: {:?}, exit: {}]",
                self.sup,
                self.restart_type,
                at,
                exit.pp()
            );

            self.sup_state = SupState::ShuttingDown(exit);
            Ok(())
        } else if let Some(idx) = self.resolve_actor_id(actor_id) {
            let ch_type = self.ch_infos[idx].ch_type;

            match (ch_type, exit.is_shutdown() || exit.is_normal()) {
                (ChildType::Transient, false) | (ChildType::Permanent, _) => (),
                (ChildType::Transient, true) | (ChildType::Temporary, _) => {
                    self.ch_states[idx] = ChState::Stopped;
                    return Ok(())
                },
            }

            let result = self.restart_intensity.report_exit(&mut self.restart_stats, at.to_owned());

            log::trace!(
                "[{}|sup:{:?}] child {:?} exited [at: {:?}; will-restart: {}; exit: {}]",
                self.sup,
                self.restart_type,
                self.ch_infos[idx].id,
                at,
                result.is_ok(),
                exit.pp()
            );

            if result.is_ok() {
                self.ch_states[idx] = ChState::ToStart;

                let ids_to_restart: VecDeque<_> = match self.restart_type {
                    RestartType::One => [].into_iter().collect(),
                    RestartType::All => self
                        .idxs()
                        .rev()
                        .filter(|i| *i != idx)
                        .map(|i| self.ch_infos[i].id)
                        .collect(),
                    RestartType::Rest => self
                        .idxs()
                        .rev()
                        .take_while(|i| *i > idx)
                        .map(|i| self.ch_infos[i].id)
                        .collect(),
                };

                log::trace!(
                    "[{}|sup:{:?}] stopping children before restart: {:?}",
                    self.sup,
                    self.restart_type,
                    ids_to_restart
                );

                match &mut self.sup_state {
                    SupState::ShuttingDown(_) => (),

                    SupState::Running | SupState::Starting => {
                        self.sup_state = SupState::Restarting(ids_to_restart);
                    },

                    SupState::Restarting(ids) => {
                        ids.extend(ids_to_restart);
                    },
                }

                Ok(())
            } else {
                self.ch_states[idx] = ChState::Stopped;

                let max_restart_intensity_reached = MaxRestartIntensityReached {
                    child_id: self.ch_infos[idx].id,
                    last_error: exit,
                };
                self.sup_state = SupState::ShuttingDown(Exit::shutdown_with_source(Arc::new(
                    max_restart_intensity_reached,
                )));

                Ok(())
            }
        } else if self.expected_exits.remove(&actor_id) {
            log::trace!(
                "[{}|sup:{:?}] received an expected exit [actor: {}, exit: {}]",
                self.sup,
                self.restart_type,
                actor_id,
                exit.pp()
            );
            Ok(())
        } else {
            log::trace!(
                "[{}|sup:{:?}] unknown linked actor exited. Shutting down [actor: {}, exit: {}]",
                self.sup,
                self.restart_type,
                actor_id,
                exit.pp()
            );
            self.sup_state = SupState::ShuttingDown(Exit::linked(actor_id, exit));
            Ok(())
        }
    }
}

#[derive(Debug)]
enum SupState<ID> {
    Running,
    Starting,
    Restarting(VecDeque<ID>),
    ShuttingDown(Exit),
}

#[derive(Debug)]
struct ChInfo<ID> {
    id: ID,
    ch_type: ChildType,
}

#[derive(Debug)]
enum ChState {
    Stopped,
    Running(ActorID),
    ToStart,
}

impl<ID, D, I> CommonDecider<ID, D, I>
where
    ID: ChildID,
{
    fn ensure_state_integrity(&self) {
        assert_eq!(self.ch_infos.len(), self.ch_states.len());
    }

    #[cfg(test)]
    pub(crate) fn expected_exits(&self) -> &HashSet<ActorID> {
        &self.expected_exits
    }

    fn idx(&self, id: ID) -> Result<usize, DeciderError> {
        self.ch_infos
            .iter()
            .enumerate()
            .find_map(|(idx, info)| Some(idx).filter(|_| info.id == id))
            .ok_or(DeciderError::UnknownId)
    }
    fn idxs(&self) -> impl DoubleEndedIterator<Item = usize> {
        self.ensure_state_integrity();

        0..self.ch_infos.len()
    }

    fn resolve_actor_id(&self, actor_id: ActorID) -> Option<usize> {
        self.ch_states.iter().enumerate().find_map(|(idx, state)| {
            Some(idx).filter(|_| matches!(state, ChState::Running(id) if *id == actor_id))
        })
    }
}
