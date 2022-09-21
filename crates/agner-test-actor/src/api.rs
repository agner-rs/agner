use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use agner_actors::{ActorID, Event, Exit, SpawnOpts, SysSpawnError, System};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::exited::Exited;
use crate::query::{ExitRq, InitAckRq, NextEventRq, Query, SetLinkRq, SetTrapExitRq};
use crate::TestActorRegistry;

#[derive(Debug, Clone)]
pub struct TestActor<M> {
    pub(crate) system: System,
    pub(crate) actor_id: ActorID,
    pub(crate) exited: Arc<Mutex<Exited>>,
    pub(crate) ctl_tx: mpsc::UnboundedSender<Query<M>>,
}

impl<M> TestActor<M> {
    pub fn prepare_args(
        registry: TestActorRegistry,
    ) -> (crate::behaviour::Args<M>, impl Future<Output = Option<ActorID>>) {
        let (init_ack_tx, init_ack_rx) = oneshot::channel();
        let (ctl_tx, ctl_rx) = mpsc::unbounded_channel();
        let args = crate::behaviour::Args::<M> { init_ack_tx, ctl_rx, ctl_tx, registry };

        (args, async move { init_ack_rx.await.ok() })
    }

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
    pub async fn post_message(&self, message: M)
    where
        M: Send + Sync + Unpin + 'static,
    {
        self.system.send(self.actor_id, message).await
    }

    pub async fn init_ack(&self, value: Option<ActorID>) {
        let (reply_on_drop, done) = oneshot::channel();
        assert!(self.ctl_tx.send(InitAckRq { value, reply_on_drop }.into()).is_ok());
        let _ = done.await;
    }

    pub async fn exit(&self, reason: Exit) {
        let (reply_on_drop, done) = oneshot::channel();
        assert!(self.ctl_tx.send(ExitRq { reason, reply_on_drop }.into()).is_ok());
        let _ = done.await;
    }

    pub async fn set_trap_exit(&self, set_to: bool) {
        let (reply_on_drop, done) = oneshot::channel();
        assert!(self.ctl_tx.send(SetTrapExitRq { set_to, reply_on_drop }.into()).is_ok());
        let _ = done.await;
    }

    pub async fn next_event(&self, timeout: Duration) -> Option<Event<M>> {
        let (reply_to, done) = oneshot::channel();
        assert!(self.ctl_tx.send(NextEventRq { timeout, reply_to }.into()).is_ok());
        done.await.ok()
    }

    pub async fn set_link(&self, actor: ActorID, link: bool) {
        let (reply_on_drop, done) = oneshot::channel();
        assert!(self.ctl_tx.send(SetLinkRq { actor, link, reply_on_drop }.into()).is_ok());
        let _ = done.await;
    }
}
