use super::*;

pub trait HListAsRef<'a> {
	type RefHList: 'a;

	fn hlist_as_ref(&'a self) -> Self::RefHList;
}

impl<'a> HListAsRef<'a> for Nil {
	type RefHList = Self;

	fn hlist_as_ref(&'a self) -> Self::RefHList {
		Self
	}
}

impl<'a, H, T> HListAsRef<'a> for Cons<H, T>
where
	T: HListAsRef<'a>,
	H: 'a,
{
	type RefHList = Cons<&'a H, T::RefHList>;

	fn hlist_as_ref(&'a self) -> Self::RefHList {
		Cons(&self.0, self.1.hlist_as_ref())
	}
}
