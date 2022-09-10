use agner_actor::{Actor, Seed, System};

use crate::static_sup::child_spec::ChildSpec;
use crate::static_sup::hlist::{HMap, HMapper};

use super::ChildCell;

pub trait ChildSpecsIntoChildren<Sys> {
	type Children;

	fn into_children(self) -> Self::Children;
}

impl<Sys, CS> ChildSpecsIntoChildren<Sys> for CS
where
	CS: HMap<ChildCellCreate<Sys>>,
{
	type Children = <CS as HMap<ChildCellCreate<Sys>>>::HMapOut;

	fn into_children(self) -> Self::Children {
		self.hlist_map(ChildCellCreate::<Sys>::default())
	}
}

#[derive(Debug, Clone, Copy)]
pub struct ChildCellCreate<Sys> {
	_pd: std::marker::PhantomData<Sys>,
	idx: usize,
}

impl<Sys> Default for ChildCellCreate<Sys> {
	fn default() -> Self {
		Self { _pd: Default::default(), idx: 0 }
	}
}

impl<'a, Sys, A, S> HMapper<ChildSpec<A, S>> for ChildCellCreate<Sys>
where
	Sys: System,
	A: Actor<Sys, Seed = S>,
	S: Seed,
{
	type Out = ChildCell<A, S>;

	fn apply(&mut self, spec: ChildSpec<A, S>) -> Self::Out {
		let ChildSpec { seed, .. } = spec;

		let cell = ChildCell::<A, S> { _pd: Default::default(), idx: self.idx, seed };
		self.idx += 1;
		cell
	}
}
