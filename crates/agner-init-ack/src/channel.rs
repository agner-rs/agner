use agner_utils::result_err_flatten::ResultErrFlattenIn;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

use agner_actors::{ActorID, Exit};

pub fn new() -> (InitAckTx, InitAckRx) {
    let (tx, rx) = oneshot::channel();
    (InitAckTx(tx), InitAckRx(rx))
}

#[derive(Debug)]
pub struct InitAckTx(oneshot::Sender<Result<ActorID, Exit>>);

#[derive(Debug)]
#[pin_project::pin_project]
pub struct InitAckRx(#[pin] oneshot::Receiver<Result<ActorID, Exit>>);

impl InitAckTx {
    pub fn ok(self, actor_id: ActorID) {
        let _ = self.0.send(Ok(actor_id));
    }

    pub fn err(self, reason: impl Into<Exit>) {
        let _ = self.0.send(Err(reason.into()));
    }

    pub fn ack(self, result: Result<ActorID, impl Into<Exit>>) {
        match result {
            Ok(actor_id) => self.ok(actor_id),
            Err(err) => self.err(err),
        }
    }
}

impl Future for InitAckRx {
    type Output = Result<ActorID, Exit>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let out = futures::ready!(this.0.poll(cx)).ok();
        let out = out.ok_or_else(Exit::no_actor).err_flatten_in();
        Poll::Ready(out)
    }
}
