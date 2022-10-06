use std::fmt;

use crate::common::gen_child_spec::traits::CreateArgs;

pub fn args_clone<T>(value: T) -> ArgsClone<T>
where
    ArgsClone<T>: CreateArgs<Input = (), Output = T>,
{
    ArgsClone(value)
}

#[derive(Clone)]
pub struct ArgsClone<T>(T);

impl<T> CreateArgs for ArgsClone<T>
where
    T: Clone,
{
    type Input = ();
    type Output = T;

    fn create_args(&mut self, (): Self::Input) -> Self::Output {
        self.0.clone()
    }
}

impl<T> fmt::Debug for ArgsClone<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsClone").field("type", &std::any::type_name::<T>()).finish()
    }
}
