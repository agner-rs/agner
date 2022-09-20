use std::sync::Arc;

use agner_actors::{ActorID, Exit, SpawnOpts, SysSpawnError, System};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

use crate::query::{ExitRq, Query};

#[derive(Debug, Clone)]
pub struct TestActor<M> {
    system: System,
    actor_id: ActorID,
    exited: Arc<Mutex<Exited>>,
    ctl_tx: mpsc::UnboundedSender<Query<M>>,
}

impl<M> TestActor<M> {
    pub async fn start(system: System, spawn_opts: SpawnOpts) -> Result<Self, SysSpawnError>
    where
        M: Send + Sync + Unpin + 'static,
    {
        let (ctl_tx, ctl_rx) = mpsc::unbounded_channel();
        let actor_id = system
            .spawn(crate::behaviour::run::<M>, crate::behaviour::Args { ctl_rx }, spawn_opts)
            .await?;
        let exited = tokio::spawn({
            let system = system.to_owned();
            async move { system.wait(actor_id).await }
        });
        let exited = Arc::new(Mutex::new(Exited::Wait(exited)));

        Ok(Self { system, actor_id, exited, ctl_tx })
    }

    pub async fn wait(&self) -> Exit {
        let mut locked = self.exited.lock().await;
        let exited = &mut *locked;
        match exited {
            Exited::Ready(reason) => reason.to_owned(),
            Exited::Wait(join_handle) => {
                let reason = join_handle.await.expect("Join failure");
                *exited = Exited::Ready(reason.to_owned());
                reason
            },
        }
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

#[derive(Debug)]
enum Exited {
    Wait(JoinHandle<Exit>),
    Ready(Exit),
}
