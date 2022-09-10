use agner_actor::{Actor, ActorID, ClonableSeed, StartOpts, System};

use crate::static_sup::hlist::{HFold, HFolder, HList, HListAsMut};
use crate::static_sup::StartError;

use super::ChildCell;

pub trait StartChildByIdx<'cell, 'sys, Sys> {
	fn start_child_by_idx(
		&'cell mut self,
		sys: &'sys Sys,
		sup_actor_id: ActorID,
		idx: usize,
	) -> Result<ActorID, StartError>;
}

impl<'sys, 'cell, Sys, CC> StartChildByIdx<'cell, 'sys, Sys> for CC
where
	Sys: 'sys,
	CC: HList,
	CC: HListAsMut<'cell>,
	<CC as HListAsMut<'cell>>::MutHList:
		HFold<ChildCellStart<'sys, Sys>, Option<Result<ActorID, StartError>>>,
{
	fn start_child_by_idx(
		&'cell mut self,
		sys: &'sys Sys,
		sup_actor_id: ActorID,
		idx: usize,
	) -> Result<ActorID, StartError> {
		self.hlist_as_mut()
			.hlist_fold(None, ChildCellStart::new(sys, sup_actor_id, idx))
			.ok_or(StartError::NotFound(idx))?
	}
}

#[derive(Debug)]
pub struct ChildCellStart<'sys, Sys> {
	sys: &'sys Sys,
	sup_actor_id: ActorID,
	idx: usize,
}
impl<'sys, Sys> ChildCellStart<'sys, Sys> {
	pub fn new(sys: &'sys Sys, sup_actor_id: ActorID, idx: usize) -> Self {
		Self { sys, sup_actor_id, idx }
	}
}

impl<'cell, 'sys, Sys, A, S>
	HFolder<Option<Result<ActorID, StartError>>, &'cell mut ChildCell<A, S>>
	for ChildCellStart<'sys, Sys>
where
	Sys: System,
	A: Actor<Sys, Seed = S>,
	S: ClonableSeed,
{
	fn apply(
		&mut self,
		acc: Option<Result<ActorID, StartError>>,
		input: &'cell mut ChildCell<A, S>,
	) -> Option<Result<ActorID, StartError>> {
		match (acc, input.idx) {
			(Some(result), _) => Some(result),
			(None, target_idx) if target_idx == self.idx => {
				let start_opts = StartOpts::new().link(self.sup_actor_id);
				match self.sys.start::<A>(input.seed.clone(), start_opts) {
					Ok(actor_id) => Some(Ok(actor_id)),
					Err(reason) => Some(Err(StartError::system_error(reason))),
				}
			},
			(None, _) => None,
		}
	}
}
