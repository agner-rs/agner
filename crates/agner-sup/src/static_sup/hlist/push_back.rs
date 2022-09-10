use super::*;

pub trait PushBack<I> {
	type Out;
	fn push_back(self, item: I) -> Self::Out;
}

impl<I> PushBack<I> for Nil {
	type Out = Cons<I, Nil>;

	fn push_back(self, item: I) -> Self::Out {
		self.push_front(item)
	}
}

impl<H, T, I> PushBack<I> for Cons<H, T>
where
	T: PushBack<I>,
{
	type Out = Cons<H, <T as PushBack<I>>::Out>;

	fn push_back(self, item: I) -> Self::Out {
		Cons(self.0, self.1.push_back(item))
	}
}
