use super::*;

pub trait HListAsMut<'a>: HListAsRef<'a> {
	type MutHList: 'a;

	fn hlist_as_mut(&'a mut self) -> Self::MutHList;
}

impl<'a> HListAsMut<'a> for Nil {
	type MutHList = Self;

	fn hlist_as_mut(&'a mut self) -> Self::MutHList {
		Self
	}
}

impl<'a, H, T> HListAsMut<'a> for Cons<H, T>
where
	T: HListAsMut<'a>,
	H: 'a,
{
	type MutHList = Cons<&'a mut H, T::MutHList>;

	fn hlist_as_mut(&'a mut self) -> Self::MutHList {
		Cons(&mut self.0, self.1.hlist_as_mut())
	}
}
