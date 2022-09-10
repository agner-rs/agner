use std::fmt;
use std::marker::PhantomData;

use agner_actor::Seed;

use super::hlist::*;

#[derive(Clone)]
pub struct SupSpec<R, L> {
	pub restart_strategy: R,
	pub children: L,
}

pub struct ChildSpec<A, S> {
	actor: PhantomData<A>,
	pub seed: S,
}

impl<A, S> Clone for ChildSpec<A, S>
where
	S: Clone,
{
	fn clone(&self) -> Self {
		Self { actor: Default::default(), seed: self.seed.clone() }
	}
}

impl<R> SupSpec<R, Nil> {
	pub fn new(restart_strategy: R) -> Self {
		Self { restart_strategy, children: Default::default() }
	}
}

impl<R, L> SupSpec<R, L> {
	pub fn child<A, S>(self, seed: S) -> SupSpec<R, L::Out>
	where
		L: PushBack<ChildSpec<A, S>>,
		S: Seed,
	{
		let Self { restart_strategy, children: list } = self;

		let list = list.push_back(ChildSpec { actor: Default::default(), seed });

		SupSpec { restart_strategy, children: list }
	}
}

impl<A, S> fmt::Debug for ChildSpec<A, S>
where
	S: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("")
			.field("actor", &std::any::type_name::<A>())
			.field("seed", &self.seed)
			.finish()
	}
}

impl<R, L> fmt::Debug for SupSpec<R, L>
where
	R: fmt::Debug,
	L: HList,
	for<'a> L: HListAsRef<'a>,
	for<'a, 'b, 'c> <L as HListAsRef<'a>>::RefHList: HFold<ChildFmtDebug, fmt::DebugStruct<'b, 'c>>,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = f.debug_struct("ChildSpec");
		s.field("restart-strategy", &self.restart_strategy);
		s.field("children-count", &L::LEN);

		let spec_ref = self.children.hlist_as_ref();
		let mut s = spec_ref.hlist_fold(s, ChildFmtDebug(0));

		s.finish()
	}
}

pub struct ChildFmtDebug(usize);

impl<'a, 'b, T> HFolder<fmt::DebugStruct<'a, 'b>, T> for ChildFmtDebug
where
	T: fmt::Debug,
{
	fn apply(&mut self, mut acc: fmt::DebugStruct<'a, 'b>, input: T) -> fmt::DebugStruct<'a, 'b> {
		acc.field(format!("[{}]", self.0).as_str(), &input);
		self.0 += 1;
		acc
	}
}
