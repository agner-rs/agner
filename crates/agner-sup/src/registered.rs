use std::sync::Arc;

use agner_actors::ActorID;
use arc_swap::ArcSwap;

#[derive(Debug, Clone, Default)]
pub struct Registered(Arc<ArcSwap<Option<ActorID>>>);

impl Registered {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn update(&self, actor_id: ActorID) {
        let _old = self.0.swap(Arc::new(Some(actor_id)));
    }
    pub fn get(&self) -> Option<ActorID> {
        **self.0.as_ref().load()
    }
}
