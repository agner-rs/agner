use std::sync::Arc;

use agner_actors::{ActorID, Exit, SpawnOpts, SysSpawnError, System};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::exited::Exited;
use crate::query::{ExitRq, Query};
use crate::TestActorRegistry;

#[derive(Debug, Clone)]
pub struct TestActor<M> {
    pub(crate) system: System,
    pub(crate) actor_id: ActorID,
    pub(crate) exited: Arc<Mutex<Exited>>,
    pub(crate) ctl_tx: mpsc::UnboundedSender<Query<M>>,
}

impl<M> TestActor<M> {
    pub async fn start(
        registry: TestActorRegistry,
        system: System,
        spawn_opts: SpawnOpts,
    ) -> Result<Self, SysSpawnError>
    where
        M: Send + Sync + Unpin + 'static,
    {
        let (init_ack_tx, init_ack_rx) = oneshot::channel();
        let (ctl_tx, ctl_rx) = mpsc::unbounded_channel();
        let actor_id = system
            .spawn(
                crate::behaviour::run::<M>,
                crate::behaviour::Args {
                    init_ack_tx,
                    ctl_rx,
                    ctl_tx: ctl_tx.to_owned(),
                    registry: registry.to_owned(),
                },
                spawn_opts,
            )
            .await?;
        let _ = init_ack_rx.await;

        Ok(registry
            .lookup(actor_id)
            .await
            .expect("Failed to lookup the actor in the registry"))
    }

    pub async fn wait(&self) -> Exit {
        crate::exited::wait(self.exited.as_ref()).await
    }

    pub fn system(&self) -> &System {
        &self.system
    }

    pub fn actor_id(&self) -> ActorID {
        self.actor_id
    }
}

impl<M> TestActor<M> {
    pub async fn exit(&self, reason: Exit) {
        let (reply_on_drop, done) = oneshot::channel();
        assert!(self.ctl_tx.send(ExitRq { reason, reply_on_drop }.into()).is_ok());
        let _ = done.await;
    }
}
