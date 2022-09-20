use std::fmt;
use std::sync::Arc;

use agner_actors::ActorID;
use arc_swap::ArcSwapWeak;

#[derive(Debug, Clone, Default)]
pub struct Service(Arc<ServiceInner>);

#[derive(Debug)]
pub struct Registered(Arc<ActorID>);

#[derive(Debug, Default)]
struct ServiceInner {
    label: Option<Box<str>>,
    actor_id: ArcSwapWeak<ActorID>,
}

impl Service {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_label(label: impl Into<Box<str>>) -> Self {
        let inner = ServiceInner { label: Some(label.into()), actor_id: Default::default() };
        Self(Arc::new(inner))
    }

    pub fn register(&self, actor_id: ActorID) -> Registered {
        let arc = Arc::new(actor_id);
        let weak = Arc::downgrade(&arc);
        let registered = Registered(arc);
        self.0.actor_id.swap(weak);
        registered
    }

    pub fn resolve(&self) -> Option<ActorID> {
        self.0.actor_id.load_full().upgrade().map(|arc| *arc)
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

#[test]
fn happy_case() {
    let id_1: ActorID = "1.0.0".parse().unwrap();
    let id_2: ActorID = "1.1.1".parse().unwrap();

    let service = Service::new();
    assert!(service.resolve().is_none());
    {
        let _registered = service.register(id_1);
        assert_eq!(service.resolve(), Some(id_1));
    }
    assert!(service.resolve().is_none());
    {
        let _registered = service.register(id_2);
        assert_eq!(service.resolve(), Some(id_2));
    }
    assert!(service.resolve().is_none());
}
