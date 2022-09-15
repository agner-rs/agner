use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

use crate::ActorID;

pub fn new() -> (InitAckTx, InitAckRx) {
    let (tx, rx) = oneshot::channel();
    (InitAckTx(tx), InitAckRx(rx))
}

#[derive(Debug)]
pub struct InitAckTx(oneshot::Sender<ActorID>);

#[derive(Debug)]
#[pin_project::pin_project]
pub struct InitAckRx(#[pin] oneshot::Receiver<ActorID>);

impl InitAckTx {
    pub fn ack(self, actor_id: ActorID) {
        let _ = self.0.send(actor_id);
    }
}

impl Future for InitAckRx {
    type Output = Option<ActorID>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let out = futures::ready!(this.0.poll(cx)).ok();
        Poll::Ready(out)
    }
}
