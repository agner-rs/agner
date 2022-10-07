use std::future::Future;

use crate::context::Context;
use crate::exit::Exit;

/// A marker trait for actor behaviour function.
///
/// It is recommended to rely on the existing implementation of this trait for certain
/// async-functions, rather than implementing this trait manually.
pub trait Actor<'a, A, M>: Send + 'static {
    type Out: Into<Exit>;
    type Fut: Future<Output = Self::Out> + Send + 'a;

    fn run(self, context: &'a mut Context<M>, args: A) -> Self::Fut;
}

impl<'a, A, M, F, Fut, Out> Actor<'a, A, M> for F
where
    M: 'a,
    F: FnOnce(&'a mut Context<M>, A) -> Fut,
    Fut: Future<Output = Out> + 'a,
    Fut: Send,
    Out: Into<Exit>,
    F: Send + 'static,
{
    type Out = Out;
    type Fut = Fut;

    fn run(self, context: &'a mut Context<M>, args: A) -> Self::Fut {
        self(context, args)
    }
}
