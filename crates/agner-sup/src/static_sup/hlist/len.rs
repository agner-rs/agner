use super::HList;

pub trait HListLen {
	fn len(&self) -> usize;
}

impl<L> HListLen for L
where
	L: HList,
{
	fn len(&self) -> usize {
		<Self as HList>::LEN
	}
}
