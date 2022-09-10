use agner_actor::error::{ExitReason, WellKnownReason};
use agner_actor::{Actor, Context, NonEmptySeed, SharedSeed, Signal, System};

mod api;
pub use api::StaticSupApi;

mod child_cell;
mod child_spec;
mod child_state;
mod hlist;
use child_state::{ChildState, RunState};

mod messages;
pub use messages::*;

pub mod restart_strategy;
pub use restart_strategy::RestartStrategy;

mod error;
use error::{StartError, StopError, SupError};

use self::child_cell::{ChildSpecsIntoChildren, StartChildByIdx};
use self::child_spec::SupSpec;
use self::hlist::HList;
use self::restart_strategy::Action;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct StaticSup<R, RS, CS, CC> {
	_pd: std::marker::PhantomData<CS>,
	restart: R,
	restart_state: RS,
	cells: CC,
	states: Vec<ChildState>,
}

#[async_trait::async_trait]
impl<Sys, R, CS, CC> Actor<Sys> for StaticSup<R, R::State, CS, CC>
where
	Sys: System,
	CC: HList,
	R: RestartStrategy,
	SupSpec<R, CS>: Clone,
	CS: Send + Sync + 'static,
	CS: ChildSpecsIntoChildren<Sys, Children = CC>,
	CC: Send + Sync + 'static,
	for<'cell, 'sys> CC: StartChildByIdx<'cell, 'sys, Sys>,
{
	type Seed = SharedSeed<SupSpec<R, CS>>;
	type Message = SupRq;

	async fn init<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		let sup_spec = context.seed().value().to_owned();

		let restart = sup_spec.restart_strategy;
		let restart_state = restart.init(CC::LEN);
		let cells = sup_spec.children.into_children();
		let states = (0..CC::LEN).map(|_| ChildState::default()).collect();

		let mut sup = Self { _pd: Default::default(), restart, restart_state, cells, states };

		for child_idx in 0..CC::LEN {
			sup.start_child(context, child_idx).await.map_err(ExitReason::error_actor)?;
			sup.restart.child_up(&mut sup.restart_state, child_idx);
			sup.process_restart_strategy_actions(context)
				.await
				.map_err(ExitReason::error_actor)?;
		}

		Ok(sup)
	}

	async fn handle_message<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		&mut self,
		_context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		match message {
			SupRq::GetChildID(idx, reply_to) => {
				let _ = reply_to.send(self.states.get(idx).and_then(|s| s.run_state.actor_id()));
			},
		}
		Ok(())
	}

	async fn handle_signal<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		signal: Signal,
	) -> Result<(), ExitReason> {
		log::trace!("StaticSup[{}] signal: {:?}", context.actor_id(), signal);

		match signal {
			Signal::Exit(who, exit_reason) => {
				if let Some((idx, state)) = self
					.states
					.iter_mut()
					.enumerate()
					.find(|(_idx, state)| state.run_state.actor_id() == Some(who))
				{
					state.run_state = RunState::Stopped;
					self.restart.child_down(&mut self.restart_state, idx, exit_reason.into());
					self.process_restart_strategy_actions(context)
						.await
						.map_err(ExitReason::error_actor)?;
					Ok(())
				} else {
					context.exit(WellKnownReason::LinkExit(who).into()).await;
					unreachable!()
				}
			},
			unhandled => {
				context.exit(WellKnownReason::unhandled_signal(unhandled).into()).await;
				unreachable!()
			},
		}
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		mut self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		log::trace!(
			"StaticSup[{}] terminating [reason: {:?}]. Shutting down children...",
			context.actor_id(),
			exit_reason
		);

		let mut wait_list = vec![];

		for child_idx in 0..CC::LEN {
			if let Some(child_id) =
				self.states.get(child_idx).and_then(|ci| ci.run_state.actor_id())
			{
				context.system().stop(child_id, ExitReason::shutdown());
				wait_list.push(child_id);
			}
		}

		let _ = futures::future::join_all(
			wait_list.into_iter().map(|child_id| context.system().wait(child_id)),
		)
		.await;

		log::trace!("StaticSup[{}] terminated", context.actor_id());
	}
}

impl<R, RS, CS, CC> StaticSup<R, RS, CS, CC> {
	async fn start_child<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		child_idx: usize,
	) -> Result<(), StartError>
	where
		CC: HList,
		CC: Send + Sync + 'static,
		for<'cell, 'sys> CC: StartChildByIdx<'cell, 'sys, Ctx::System>,
	{
		let state = self.states.get_mut(child_idx).ok_or(StartError::NotFound(child_idx))?;
		if state.run_state.actor_id().is_some() {
			return Err(StartError::AlreadyStarted)
		}
		let child_id =
			self.cells.start_child_by_idx(context.system(), context.actor_id(), child_idx)?;
		state.run_state = RunState::Running(child_id);

		Ok(())
	}

	async fn stop_child<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
		child_idx: usize,
		reason: ExitReason,
	) -> Result<(), StopError> {
		let state = self.states.get_mut(child_idx).ok_or(StopError::NotFound(child_idx))?;
		let child_id = state.run_state.actor_id().ok_or(StopError::AlreadyStopped)?;
		context.system().stop(child_id, reason);
		let _ = context.system().wait(child_id).await;
		state.run_state = RunState::Stopped;

		Ok(())
	}

	async fn process_restart_strategy_actions<Ctx: Context>(
		&mut self,
		context: &mut Ctx,
	) -> Result<(), SupError>
	where
		R: RestartStrategy<State = RS>,
		CC: HList,
		CC: Send + Sync + 'static,
		for<'cell, 'sys> CC: StartChildByIdx<'cell, 'sys, Ctx::System>,
	{
		while let Some(action) = self.restart.next_action(&mut self.restart_state) {
			match action {
				Action::Start { child } => {
					log::trace!("starting child[{}]...", child);
					self.start_child(context, child).await?
				},
				Action::Stop { child, reason } => {
					log::trace!("stopping child[{}] with [reason: {}]", child, reason);
					self.stop_child(context, child, reason).await?
				},
				Action::Exit { reason } => {
					log::trace!("shutting down supervisor [reason: {}]", reason);
					context.exit(reason).await;
					unreachable!()
				},
			}
		}

		Ok(())
	}
}
