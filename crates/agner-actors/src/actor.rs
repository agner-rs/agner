use std::convert::Infallible;
use std::future::Future;

use crate::context::Context;
use crate::exit::Exit;
use crate::ArcError;

pub trait Actor<'a, A, M>: Send + Sync + 'static {
    type Out: IntoExitReason;
    type Fut: Future<Output = Self::Out> + Send + Sync + 'a;

    fn run(self, context: &'a mut Context<M>, arg: A) -> Self::Fut;
}

pub trait IntoExitReason {
    fn into_exit_reason(self) -> Exit;
}

impl IntoExitReason for Exit {
    fn into_exit_reason(self) -> Exit {
        self
    }
}
impl IntoExitReason for () {
    fn into_exit_reason(self) -> Exit {
        Default::default()
    }
}
impl IntoExitReason for Infallible {
    fn into_exit_reason(self) -> Exit {
        unreachable!("Whoa! An instance of {}: {:?}", std::any::type_name::<Self>(), self)
    }
}

impl<E> IntoExitReason for Result<(), E>
where
    E: Into<ArcError>,
{
    fn into_exit_reason(self) -> Exit {
        if let Err(reason) = self {
            Exit::custom(reason.into())
        } else {
            Exit::normal()
        }
    }
}

impl<'a, A, M, F, Fut, Out> Actor<'a, A, M> for F
where
    M: 'a,
    F: FnOnce(&'a mut Context<M>, A) -> Fut,
    Fut: Future<Output = Out> + 'a,
    Fut: Send + Sync,
    Out: IntoExitReason,
    F: Send + Sync + 'static,
{
    type Out = Out;
    type Fut = Fut;

    fn run(self, context: &'a mut Context<M>, arg: A) -> Self::Fut {
        self(context, arg)
    }
}

// impl<'a, A, M, F, Fut, Out> Actor<'a, A, M> for F
// where
// 	M: 'a,
// 	F: Fn(&'a mut Context<M>, A) -> Fut,
// 	Fut: Future<Output = Out> + 'a,
// 	Fut: Send + Sync,
// 	Out: IntoExitReason,
// {
// 	type Out = Out;
// 	type Fut = Fut;

// 	fn run(self, context: &'a mut Context<M>, arg: A) -> Self::Fut {
// 		self(context, arg)
// 	}
// }
