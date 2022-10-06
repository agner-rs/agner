use std::fmt;
use std::sync::Arc;

use agner_actors::ActorID;
use arc_swap::ArcSwapWeak;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, Default)]
pub struct Service(Arc<ServiceInner>);

#[derive(Debug)]
pub struct Registered(Arc<ActorID>);

#[derive(Debug, Default)]
struct ServiceInner {
    label: Option<Box<str>>,
    actor_id: ArcSwapWeak<ActorID>,
    waiting: Mutex<Vec<oneshot::Sender<ActorID>>>,
}

impl Service {
    pub fn new() -> Self {
        Default::default()
    }

    #[deprecated(since = "0.3.5")]
    pub fn new_with_label(_label: impl Into<Box<str>>) -> Self {
        Self::new()
    }
    pub async fn register(&self, actor_id: ActorID) -> Registered {
        let arc = Arc::new(actor_id);
        let weak = Arc::downgrade(&arc);
        let registered = Registered(arc);
        self.0.actor_id.swap(weak);
        self.0.waiting.lock().await.drain(..).for_each(|tx| {
            let _ = tx.send(actor_id);
        });

        registered
    }

    pub fn resolve(&self) -> Option<ActorID> {
        self.0.actor_id.load_full().upgrade().map(|arc| *arc)
    }

    pub async fn wait(&self) -> Option<ActorID> {
        if let Some(registered) = self.resolve() {
            return Some(registered)
        }

        let (tx, rx) = oneshot::channel();

        let mut waiting = self.0.waiting.lock().await;
        let maybe_replace = waiting.iter_mut().enumerate().find(|(_idx, tx)| tx.is_closed());
        if let Some((_idx, to_replace)) = maybe_replace {
            *to_replace = tx;
        } else {
            waiting.push(tx);
        }
        std::mem::drop(waiting);
        if let Some(registered) = self.resolve() {
            Some(registered)
        } else {
            rx.await.ok()
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Service");

        if let Some(id) = self.resolve() {
            s.field("id", &id);
        }
        if let Some(label) = self.0.label.as_ref() {
            s.field("label", label);
        }

        s.finish()
    }
}

impl fmt::Display for Registered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Registered[{}]", self.0.as_ref())
    }
}

#[tokio::test]
async fn happy_case() {
    let id_1: ActorID = "1.0.0".parse().unwrap();
    let id_2: ActorID = "1.1.1".parse().unwrap();

    let service = Service::new();
    assert!(service.resolve().is_none());
    {
        let _registered = service.register(id_1).await;
        assert_eq!(service.resolve(), Some(id_1));
    }
    assert!(service.resolve().is_none());
    {
        let _registered = service.register(id_2).await;
        assert_eq!(service.resolve(), Some(id_2));
    }
    assert!(service.resolve().is_none());
}
