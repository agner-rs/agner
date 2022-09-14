use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;

use crate::context::Context;
use crate::exit_reason::ExitReason;

pub trait Actor<'a, A, M> {
	type Out: IntoExitReason;
	type Fut: Future<Output = Self::Out> + Send + Sync + 'a;

	fn run(self, context: &'a mut Context<M>, arg: A) -> Self::Fut;
}

pub trait IntoExitReason {
	fn into_exit_reason(self) -> ExitReason;
}

impl IntoExitReason for ExitReason {
	fn into_exit_reason(self) -> ExitReason {
		self
	}
}
impl IntoExitReason for () {
	fn into_exit_reason(self) -> ExitReason {
		Default::default()
	}
}
impl IntoExitReason for Infallible {
	fn into_exit_reason(self) -> ExitReason {
		unreachable!("Whoa! An instance of {}: {:?}", std::any::type_name::<Self>(), self)
	}
}
impl<E> IntoExitReason for Result<(), E>
where
	E: std::error::Error + Send + Sync + 'static,
{
	fn into_exit_reason(self) -> ExitReason {
		if let Err(reason) = self {
			ExitReason::Generic(Arc::new(reason))
		} else {
			ExitReason::Normal
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
