use agner_actor::{NonEmptySeed, Seed, SeedMut, SharedSeed, UniqueSeed};

use super::seed_factory::SeedFactoryMut;
use super::SeedFactory;

pub trait DynSupSeed<In, Out>: Seed {
	fn produce_seed(&mut self, rq: In) -> Option<Out>;
}

impl<WSF, In, Out> DynSupSeed<In, Out> for UniqueSeed<WSF>
where
	WSF: SeedFactoryMut<In, Out>,
	In: Send + Sync + 'static,
	Out: Seed,
{
	fn produce_seed(&mut self, rq: In) -> Option<Out> {
		self.value_mut().map(move |f| f.seed(rq))
	}
}

impl<WSF, In, Out> DynSupSeed<In, Out> for SharedSeed<WSF>
where
	WSF: SeedFactory<In, Out>,
	In: Send + Sync + 'static,
	Out: Seed,
{
	fn produce_seed(&mut self, rq: In) -> Option<Out> {
		Some(self.value().seed(rq))
	}
}
