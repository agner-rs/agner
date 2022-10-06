use std::fmt;
use std::marker::PhantomData;

use crate::common::gen_child_spec::traits::CreateArgs;

pub fn args_call0<F, Out>(f: F) -> ArgsCallFn0<F, Out>
where
    ArgsCallFn0<F, Out>: CreateArgs<Input = (), Output = Out>,
{
    ArgsCallFn0(f, Default::default())
}

pub fn args_call1<F, In, Out>(f: F) -> ArgsCallFn1<F, In, Out>
where
    ArgsCallFn1<F, In, Out>: CreateArgs<Input = In, Output = Out>,
{
    ArgsCallFn1(f, Default::default())
}

pub struct ArgsCallFn0<F, Out>(F, PhantomData<Out>);

pub struct ArgsCallFn1<F, In, Out>(F, PhantomData<(In, Out)>);

impl<F, Out> CreateArgs for ArgsCallFn0<F, Out>
where
    F: FnMut() -> Out,
{
    type Input = ();
    type Output = Out;

    fn create_args(&mut self, (): Self::Input) -> Self::Output {
        (self.0)()
    }
}

impl<F, In, Out> CreateArgs for ArgsCallFn1<F, In, Out>
where
    F: FnMut(In) -> Out,
{
    type Input = In;
    type Output = Out;

    fn create_args(&mut self, input: Self::Input) -> Self::Output {
        (self.0)(input)
    }
}

impl<F, Out> fmt::Debug for ArgsCallFn0<F, Out>
where
    F: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsCallFn0")
            .field("out", &std::any::type_name::<Out>())
            .field("func", &self.0)
            .finish()
    }
}

impl<F, In, Out> fmt::Debug for ArgsCallFn1<F, In, Out>
where
    F: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsCallFn1")
            .field("in", &std::any::type_name::<In>())
            .field("out", &std::any::type_name::<Out>())
            .field("func", &self.0)
            .finish()
    }
}

impl<F, Out> Clone for ArgsCallFn0<F, Out>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.to_owned(), Default::default())
    }
}

impl<F, In, Out> Clone for ArgsCallFn1<F, In, Out>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.to_owned(), Default::default())
    }
}
