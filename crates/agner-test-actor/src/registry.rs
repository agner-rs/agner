use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use agner_actors::{ActorID, System};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::exited::Exited;
use crate::query::Query;
use crate::TestActor;

#[derive(Debug, Clone, Default)]
pub struct TestActorRegistry(pub Arc<RwLock<HashMap<ActorID, TestActorEntry>>>);

impl TestActorRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn lookup<M>(&self, actor_id: ActorID) -> Option<TestActor<M>>
    where
        M: Send + Sync + 'static,
    {
        self.0.read().await.get(&actor_id).and_then(|entry| {
            let ctl_tx = entry.ctl_tx.downcast_ref::<mpsc::UnboundedSender<Query<M>>>().cloned()?;
            let test_actor = TestActor {
                system: entry.system.to_owned(),
                actor_id,
                exited: entry.exited.to_owned(),
                ctl_tx,
            };
            Some(test_actor)
        })
    }
}

#[derive(Debug)]
pub struct TestActorEntry {
    pub system: System,
    pub exited: Arc<Mutex<Exited>>,
    pub ctl_tx: Box<dyn Any + Send + Sync + 'static>,
}
