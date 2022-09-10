mod fold;
pub use fold::*;

mod as_ref;
pub use as_ref::*;

mod as_mut;
pub use as_mut::*;

mod push_back;
pub use push_back::*;

mod map;
pub use map::*;

mod len;
pub use len::HListLen;

#[derive(Debug, Clone, Copy, Default)]
pub struct Nil;

#[derive(Debug, Clone)]
pub struct Cons<H, T>(H, T);

pub trait HList: Sized {
	type Head;
	type Tail: HList;

	const LEN: usize;

	fn push_front<H>(self, head: H) -> Cons<H, Self> {
		Cons(head, self)
	}

	fn head(&self) -> &Self::Head;
	fn tail(&self) -> Option<&Self::Tail>;
}

impl HList for Nil {
	const LEN: usize = 0;
	type Head = std::convert::Infallible;
	type Tail = Self;

	fn head(&self) -> &Self::Head {
		panic!("head of nil");
	}
	fn tail(&self) -> Option<&Self::Tail> {
		None
	}
}

impl<H, T> HList for Cons<H, T>
where
	T: HList,
{
	const LEN: usize = 1 + T::LEN;
	type Head = H;
	type Tail = T;

	fn head(&self) -> &Self::Head {
		&self.0
	}
	fn tail(&self) -> Option<&Self::Tail> {
		Some(&self.1)
	}
}
