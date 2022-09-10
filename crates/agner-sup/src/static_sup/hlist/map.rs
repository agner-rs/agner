use super::*;

pub trait HMap<M> {
	type HMapOut;

	fn hlist_map(self, mapper: M) -> Self::HMapOut;
}

pub trait HMapper<In> {
	type Out;

	fn apply(&mut self, input: In) -> Self::Out;
}

impl<M> HMap<M> for Nil {
	type HMapOut = Self;

	fn hlist_map(self, _mapper: M) -> Self::HMapOut {
		Self
	}
}

impl<M, HIn, T> HMap<M> for Cons<HIn, T>
where
	M: HMapper<HIn>,
	T: HMap<M>,
{
	type HMapOut = Cons<M::Out, T::HMapOut>;

	fn hlist_map(self, mut mapper: M) -> Self::HMapOut {
		let head = mapper.apply(self.0);
		let tail = self.1.hlist_map(mapper);

		Cons(head, tail)
	}
}
