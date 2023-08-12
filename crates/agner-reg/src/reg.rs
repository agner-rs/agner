use std::sync::Arc;

use agner_actors::ActorID;
use tokio::sync::watch;

pub fn new() -> (RegTx, RegRx) {
    let (tx, rx) = watch::channel(None);
    let tx = RegTx(Arc::new(tx));
    let rx = RegRx(rx);
    (tx, rx)
}

#[derive(Debug, Clone)]
pub struct RegRx(watch::Receiver<State>);

#[derive(Debug, Clone)]
pub struct RegTx(RegTxInner);

#[derive(Debug)]
pub struct RegGuard {
    tx_inner: RegTxInner,
    actor_id: ActorID,
}

impl RegTx {
    pub fn register(&self, actor_id: ActorID) -> RegGuard {
        RegGuard::new(Arc::clone(&self.0), actor_id)
    }
}

impl RegRx {
    pub fn resolve(&self) -> Option<ActorID> {
        *self.0.borrow()
    }

    pub async fn wait(&mut self) -> Option<ActorID> {
        loop {
            if let Some(actor_id) = *self.0.borrow() {
                break Some(actor_id)
            }

            if self.0.changed().await.is_err() {
                break None
            }
        }
    }
}

type State = Option<ActorID>;
type RegTxInner = Arc<watch::Sender<State>>;

impl RegGuard {
    fn new(tx_inner: RegTxInner, actor_id: ActorID) -> Self {
        tx_inner.send_modify(|v| *v = Some(actor_id));
        Self { tx_inner, actor_id }
    }
}

impl Drop for RegGuard {
    fn drop(&mut self) {
        self.tx_inner.send_modify(|v| {
            if matches!(v, Some(this) if *this == self.actor_id) {
                *v = None;
            }
        });
    }
}
