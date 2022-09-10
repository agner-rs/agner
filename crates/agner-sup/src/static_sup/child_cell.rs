use std::fmt;

mod create;
pub use create::ChildSpecsIntoChildren;

mod start;
pub use start::StartChildByIdx;

pub struct ChildCell<A, S> {
	_pd: std::marker::PhantomData<A>,
	idx: usize,
	seed: S,
}

impl<A, S> fmt::Debug for ChildCell<A, S> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ChildCell<{}, {}>", std::any::type_name::<A>(), std::any::type_name::<S>())
	}
}
