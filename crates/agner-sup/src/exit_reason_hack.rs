use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use agner_actors::{Actor, Context as ActorContext, ExitReason, IntoExitReason};

pub fn adapt_exit_reason<A, M>(
    behaviour: impl for<'a> Actor<'a, A, M> + Clone,
) -> impl for<'a> Actor<'a, A, M> + Clone {
    NormalizeExitReason(behaviour)
}

#[derive(Debug, Clone)]
struct NormalizeExitReason<B>(B);

#[pin_project::pin_project]
struct OutputNormalized<F>(#[pin] F);

impl<F> Future for OutputNormalized<F>
where
    F: Future,
    F::Output: IntoExitReason,
{
    type Output = ExitReason;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let output = futures::ready!(this.0.poll(cx));
        let original_exit_reason = output.into_exit_reason();
        let exit_reason = if original_exit_reason.is_normal() {
            ExitReason::shutdown()
        } else {
            original_exit_reason
        };
        Poll::Ready(exit_reason)
    }
}

impl<'a, B, A, M> Actor<'a, A, M> for NormalizeExitReason<B>
where
    B: Actor<'a, A, M>,
{
    type Fut = OutputNormalized<B::Fut>;
    type Out = ExitReason;

    fn run(self, context: &'a mut ActorContext<M>, arg: A) -> Self::Fut {
        OutputNormalized(self.0.run(context, arg))
    }
}
