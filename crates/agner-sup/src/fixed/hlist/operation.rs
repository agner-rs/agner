use agner_actors::Context;

use crate::fixed::hlist::index::{Here, Index, IndexMut, There};
use crate::fixed::ChildSpec;

pub trait ApplyMut<O, I> {
    type Out;

    fn apply(&mut self, operation: &mut O) -> Self::Out;
}

pub trait OpMut<In> {
    type Out;
    fn apply(&mut self, input: &mut In) -> Self::Out;
}

impl<O, I, L> ApplyMut<O, I> for L
where
    L: IndexMut<I>,
    O: OpMut<<L as Index<I>>::Item>,
{
    type Out = O::Out;

    fn apply(&mut self, operation: &mut O) -> Self::Out {
        operation.apply(self.get_mut())
    }
}

// pub struct Start<'a, M> {
//     context: &'a mut Context<M>,
// }
// impl<'a, M, CS, CM> OpMut<CS> for Start<'a, M> where CS: ChildSpec<CM> {

// }
