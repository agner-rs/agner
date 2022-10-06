use std::fmt;

use crate::common::gen_child_spec::traits::CreateArgs;

pub fn args_unique<T>(value: T) -> ArgsUnique<T>
where
    ArgsUnique<T>: CreateArgs<Input = (), Output = Option<T>>,
{
    ArgsUnique(Some(value))
}

pub struct ArgsUnique<T>(Option<T>);

impl<T> CreateArgs for ArgsUnique<T> {
    type Input = ();
    type Output = Option<T>;

    fn create_args(&mut self, (): Self::Input) -> Self::Output {
        self.0.take()
    }
}

impl<T> fmt::Debug for ArgsUnique<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsClone")
            .field("type", &std::any::type_name::<T>())
            .field("is_some", &self.0.is_some())
            .finish()
    }
}
