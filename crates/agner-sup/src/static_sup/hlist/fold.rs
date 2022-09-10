use super::*;

pub trait HFold<F, Acc> {
	fn hlist_fold(self, acc: Acc, folder: F) -> Acc;
}

pub trait HFolder<Acc, In> {
	fn apply(&mut self, acc: Acc, input: In) -> Acc;
}

impl<M, Acc> HFold<M, Acc> for Nil {
	fn hlist_fold(self, acc: Acc, _map: M) -> Acc {
		acc
	}
}
impl<M, Acc, H, T> HFold<M, Acc> for Cons<H, T>
where
	M: HFolder<Acc, H>,
	T: HFold<M, Acc>,
{
	fn hlist_fold(self, acc: Acc, mut folder: M) -> Acc {
		let acc = folder.apply(acc, self.0);
		self.1.hlist_fold(acc, folder)
	}
}
