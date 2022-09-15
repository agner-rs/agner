use crate::fixed::hlist::{Cons, HList, Nil};

pub trait ApplyMut<O, Out> {
    fn apply_mut(&mut self, index: usize, len: usize, operation: &mut O) -> Out;
}

pub trait OpMut<In> {
    type Out;
    fn apply_mut(&mut self, input: &mut In) -> Self::Out;
}

impl<O, Out> ApplyMut<O, Out> for Nil {
    fn apply_mut(&mut self, index: usize, len: usize, operation: &mut O) -> Out {
        panic!("index out of range");
    }
}
impl<H, T, O, Out> ApplyMut<O, Out> for Cons<H, T>
where
    Self: HList,
    O: OpMut<H, Out = Out>,
    T: ApplyMut<O, Out>,
{
    fn apply_mut(&mut self, index: usize, len: usize, operation: &mut O) -> Out {
        if len - index == Self::LEN {
            operation.apply_mut(&mut self.0)
        } else {
            self.1.apply_mut(index, len, operation)
        }
    }
}
