use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Weak};

use futures::{stream, Stream, StreamExt};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::actor::Actor;
use crate::actor_id::ActorID;
use crate::actor_runner::sys_msg::SysMsg;
use crate::actor_runner::ActorRunner;
use crate::spawn_opts::SpawnOpts;
use crate::system_config::SystemConfig;
use crate::{ActorInfo, ExitReason};

mod actor_entry;
use actor_entry::ActorEntry;

mod actor_id_pool;
use actor_id_pool::ActorIDPool;

mod errors;
pub use errors::{SysChannelError, SysSpawnError};

#[derive(Debug, Clone)]
pub struct System(Arc<Inner>);

impl System {
    pub(crate) fn rc_downgrade(&self) -> SystemOpt {
        SystemOpt(Arc::downgrade(&self.0))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SystemOpt(Weak<Inner>);
impl SystemOpt {
    pub(crate) fn rc_upgrade(&self) -> Option<System> {
        self.0.upgrade().map(System)
    }
}

impl System {
    /// Create a new [`System`] using the provided config.
    pub fn new(config: SystemConfig) -> Self {
        static NEXT_SYSTEM_ID: AtomicUsize = AtomicUsize::new(1);

        let system_id = NEXT_SYSTEM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let actor_id_pool = ActorIDPool::new(system_id, config.max_actors);
        let actor_entries = (0..config.max_actors).map(|_| RwLock::new(None)).collect();

        let inner = Inner { config, system_id, actor_id_pool, actor_entries };
        Self(Arc::new(inner))
    }

    /// The config with which this [`System`] was created.
    pub fn config(&self) -> &SystemConfig {
        &self.0.config
    }
}

impl System {
    /// Spawn an actor
    ///
    /// Example:
    /// ```
    /// use agner_actors::{System, Context, Event};
    ///
    /// async fn actor_behaviour(context: &mut Context<&'static str>, actor_name: &'static str) {
    /// 	loop {
    /// 		if let Event::Message(message) = context.next_event().await {
    /// 			eprintln!("[{}] received: {}", actor_name, message);
    /// 		}
    /// 	}
    /// }
    /// let _ = async {
    /// 	let system = System::new(Default::default());
    ///
    /// 	let alice = system.spawn(actor_behaviour, "Alice", Default::default()).await.expect("Failed to spawn an actor");
    /// 	let bob = system.spawn(actor_behaviour, "Bob", Default::default()).await.expect("Failed to spawn an actor");
    /// };
    /// ```
    pub async fn spawn<Behaviour, Arg, Message>(
        &self,
        behaviour: Behaviour,
        arg: Arg,
        spawn_opts: SpawnOpts,
    ) -> Result<ActorID, SysSpawnError>
    where
        Arg: Send + Sync + 'static,
        Message: Unpin + Send + Sync + 'static,
        for<'a> Behaviour: Actor<'a, Arg, Message>,
        // for<'a> <Behaviour as Actor<'a, Arg, Message>>::Fut: Send + Sync,
    {
        let system = self.to_owned();
        let actor_id_lease =
            system.0.actor_id_pool.acquire_id().ok_or(SysSpawnError::MaxActorsLimit)?;
        let actor_id = *actor_id_lease;

        let (messages_tx, messages_rx) = mpsc::unbounded_channel::<Message>();
        let (sys_msg_tx, sys_msg_rx) = mpsc::unbounded_channel();

        let actor = ActorRunner {
            actor_id,
            system_opt: system.rc_downgrade(),
            messages_rx,
            sys_msg_rx,
            sys_msg_tx: sys_msg_tx.to_owned(),
            spawn_opts,
        };
        tokio::spawn(actor.run(behaviour, arg));

        let entry = ActorEntry { actor_id_lease, messages_tx: Box::new(messages_tx), sys_msg_tx };

        self.actor_entry_put(entry).await;

        Ok(actor_id)
    }

    /// Send SigExit to the specified actor.
    pub async fn exit(&self, actor_id: ActorID, exit_reason: ExitReason) {
        self.send_sys_msg(actor_id, SysMsg::SigExit(actor_id, exit_reason)).await;
    }

    /// Wait for the specified actor to terminate, and return upon its termination the
    /// [`ExitReason`]. In case the actor with the specified `actor_id` does not exist — return
    /// [`ExitReason::NoProcess`] right away.
    pub async fn wait(&self, actor_id: ActorID) -> ExitReason {
        let (tx, rx) = oneshot::channel();
        if self.send_sys_msg(actor_id, SysMsg::Wait(tx)).await {
            rx.await.unwrap_or_else(|_| ExitReason::NoActor)
        } else {
            ExitReason::NoActor
        }
    }

    /// Send a [`SysMsg`] to the specified process.
    /// Returns `true` if both:
    /// - the process entry corresponding to the `to` existed;
    /// - the underlying mpsc-channel accepted the message (i.e. was not closed before this message
    ///   is sent).
    pub(crate) async fn send_sys_msg(&self, to: ActorID, sys_msg: SysMsg) -> bool {
        self.actor_entry_read(to, |entry| entry.sys_msg_tx.send(sys_msg).ok())
            .await
            .flatten()
            .is_some()
    }

    /// Send a single message to the specified actor.
    pub async fn send<M>(&self, actor_id: ActorID, message: M)
    where
        M: 'static,
    {
        let _ = self
            .actor_entry_read(actor_id, move |e| {
                e.messages_tx
                    .downcast_ref::<mpsc::UnboundedSender<M>>()
                    .map(move |tx| tx.send(message))
            })
            .await;
    }

    /// Open a channel to the specified actor.
    pub async fn channel<M>(
        &self,
        actor_id: ActorID,
    ) -> Result<mpsc::UnboundedSender<M>, SysChannelError>
    where
        M: 'static,
    {
        let chan = self
            .actor_entry_read(actor_id, |e| {
                e.messages_tx.downcast_ref::<mpsc::UnboundedSender<M>>().map(ToOwned::to_owned)
            })
            .await
            .ok_or(SysChannelError::NoActor)?
            .ok_or(SysChannelError::InvalidMessageType)?;

        Ok(chan)
    }

    /// Link two actors
    pub async fn link(&self, left: ActorID, right: ActorID) {
        let left_accepted_sys_msg = self.send_sys_msg(left, SysMsg::Link(right)).await;
        let right_accepted_sys_msg = self.send_sys_msg(right, SysMsg::Link(left)).await;

        if !right_accepted_sys_msg {
            self.send_sys_msg(left, SysMsg::SigExit(right, ExitReason::NoActor)).await;
        }
        if !left_accepted_sys_msg {
            self.send_sys_msg(right, SysMsg::SigExit(left, ExitReason::NoActor)).await;
        }
    }

    pub fn all_actors<'a>(&'a self) -> impl Stream<Item = ActorID> + 'a {
        stream::iter(&self.0.actor_entries[..]).filter_map(|slot| async move {
            let locked = slot.read().await;
            locked.as_ref().map(|entry| *entry.actor_id_lease)
        })
    }

    pub async fn actor_info(&self, actor_id: ActorID) -> Option<ActorInfo> {
        let (tx, rx) = oneshot::channel();
        self.send_sys_msg(actor_id, SysMsg::GetInfo(tx)).await;
        rx.await.ok()
    }
}

#[derive(Debug)]
struct Inner {
    config: SystemConfig,
    system_id: usize,
    actor_id_pool: ActorIDPool,
    actor_entries: Box<[RwLock<Option<ActorEntry>>]>,
}
