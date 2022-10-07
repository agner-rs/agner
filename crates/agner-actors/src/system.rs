use std::any::Any;
use std::future::Future;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Weak};

use futures::{stream, Stream, StreamExt};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::actor::Actor;
use crate::actor_id::ActorID;
use crate::actor_runner::sys_msg::{ActorInfo, SysMsg};
use crate::actor_runner::ActorRunner;
use crate::exit::Exit;
use crate::exit_handler::ExitHandler;
use crate::spawn_opts::SpawnOpts;
use crate::system_config::SystemConfig;

mod actor_entry;
mod sys_actor_entry;
use actor_entry::ActorEntry;

mod actor_id_pool;
use actor_id_pool::ActorIDPool;

mod errors;
pub use errors::{SysChannelError, SysSpawnError};

/// A [`System`](crate::system::System) is a scope within which the actors run.
#[derive(Debug, Clone)]
pub struct System(Arc<Inner>);

impl System {
    pub fn rc_downgrade(&self) -> SystemWeakRef {
        SystemWeakRef(Arc::downgrade(&self.0))
    }
}

#[derive(Debug, Clone)]
pub struct SystemWeakRef(Weak<Inner>);
impl SystemWeakRef {
    pub fn rc_upgrade(&self) -> Option<System> {
        self.0.upgrade().map(System)
    }
}

impl System {
    /// Create a new [`System`] using the provided config.
    pub fn new(config: SystemConfig) -> Self {
        static NEXT_SYSTEM_ID: AtomicUsize = AtomicUsize::new(1);

        let system_id = NEXT_SYSTEM_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let actor_id_pool = ActorIDPool::new(system_id, config.max_actors);
        let actor_entries =
            (0..config.max_actors).map(|_| RwLock::new(Default::default())).collect();

        let exit_handler = config.exit_handler.to_owned();

        let inner = Inner { config, system_id, actor_id_pool, actor_entries, exit_handler };
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
    ///     loop {
    ///         if let Event::Message(message) = context.next_event().await {
    ///             eprintln!("[{}] received: {}", actor_name, message);
    ///         }
    ///     }
    /// }
    /// let _ = async {
    ///     let system = System::new(Default::default());
    ///
    ///     let alice = system.spawn(actor_behaviour, "Alice", Default::default()).await.expect("Failed to spawn an actor");
    ///     let bob = system.spawn(actor_behaviour, "Bob", Default::default()).await.expect("Failed to spawn an actor");
    /// };
    /// ```
    pub async fn spawn<Behaviour, Args, Message>(
        &self,
        behaviour: Behaviour,
        args: Args,
        mut spawn_opts: SpawnOpts,
    ) -> Result<ActorID, SysSpawnError>
    where
        Args: Send + 'static,
        Message: Unpin + Send + 'static,
        for<'a> Behaviour: Actor<'a, Args, Message>,
    {
        let exit_handler =
            spawn_opts.take_exit_handler().unwrap_or_else(|| self.0.exit_handler.to_owned());

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
            exit_handler,
            spawn_opts,
        };
        tokio::spawn(actor.run(behaviour, args));

        let entry = ActorEntry::new(actor_id_lease, messages_tx, sys_msg_tx);
        // let entry = ActorEntryOld { actor_id_lease, messages_tx: Box::new(messages_tx),
        // sys_msg_tx };

        self.actor_entry_put(entry).await;

        Ok(actor_id)
    }

    /// Send SigExit to the specified actor.
    pub async fn exit(&self, actor_id: ActorID, exit_reason: Exit) {
        self.send_sys_msg(actor_id, SysMsg::SigExit(actor_id, exit_reason)).await;
    }

    /// Wait for the specified actor to terminate, and return upon its termination the
    /// [`Exit`](crate::exit::Exit). In case the actor with the specified `actor_id` does not exist
    /// — return [`Exit::no_actor()`](`crate::exit::Exit::no_actor`) right away.
    pub fn wait(&self, actor_id: ActorID) -> impl Future<Output = Exit> {
        let sys = self.clone();
        async move {
            let (tx, rx) = oneshot::channel();

            if let Some(mut entry) = sys.actor_entry_write(actor_id).await {
                entry.add_watch(tx);
            } else {
                log::warn!("attempt to install a watch before the ActorEntry is initialized [actor_id: {}]", actor_id);
            }
            rx.await.unwrap_or_else(|_| Exit::no_actor())
        }
    }

    /// Send a [`SysMsg`] to the specified process.
    /// Returns `true` if both:
    /// - the process entry corresponding to the `to` existed;
    /// - the underlying mpsc-channel accepted the message (i.e. was not closed before this message
    ///   is sent).
    pub(crate) async fn send_sys_msg(&self, to: ActorID, sys_msg: SysMsg) -> bool {
        log::trace!(
            "[sys:{}] trying to send sys-msg [to: {}, sys-msg: {:?}]",
            self.0.system_id,
            to,
            sys_msg
        );

        if let Some(entry) = self.actor_entry_read(to).await {
            if entry.running_actor_id() == Some(to) {
                if let Some(tx) = entry.sys_msg_tx() {
                    return tx.send(sys_msg).is_ok()
                }
            }
        }
        false
    }

    /// Send a single message to the specified actor.
    pub async fn send<M>(&self, to: ActorID, message: M)
    where
        M: Send + 'static,
    {
        log::trace!(
            "[sys:{}] trying to send message [to: {}, msg-type: {}]",
            self.0.system_id,
            to,
            std::any::type_name::<M>()
        );
        if let Some(entry) = self.actor_entry_read(to).await {
            if entry.running_actor_id() == Some(to) {
                if let Some(tx) = entry.messages_tx::<M>() {
                    let _ = tx.send(message);
                }
            }
        }
    }

    /// Open a channel to the specified actor.
    ///
    /// When sending a series of messages to an actor, it may be better from the performance point
    /// of view to open a channel to an actor, rather than sending each message separately using
    /// [`System::send::<Message>(&self, ActorID, Message)`](crate::system::System::send).
    pub async fn channel<M>(&self, to: ActorID) -> Result<mpsc::UnboundedSender<M>, SysChannelError>
    where
        M: Send + 'static,
    {
        self.actor_entry_read(to)
            .await
            .ok_or(SysChannelError::NoActor)?
            .messages_tx()
            .cloned()
            .ok_or(SysChannelError::InvalidMessageType)
    }

    /// Link two actors
    pub async fn link(&self, left: ActorID, right: ActorID) {
        let left_accepted_sys_msg = self.send_sys_msg(left, SysMsg::Link(right)).await;
        let right_accepted_sys_msg = self.send_sys_msg(right, SysMsg::Link(left)).await;

        if !right_accepted_sys_msg {
            self.send_sys_msg(left, SysMsg::SigExit(right, Exit::no_actor())).await;
        }
        if !left_accepted_sys_msg {
            self.send_sys_msg(right, SysMsg::SigExit(left, Exit::no_actor())).await;
        }
    }

    /// Associate arbitrary data with the specified actor.
    /// Upon actor termination that data will be dropped.
    /// If no actor with the specified id exists, the data will be dropped right away.
    pub async fn put_data<D: Any + Send + Sync + 'static>(&self, actor_id: ActorID, data: D) {
        if let Some(mut actor_entry) = self.actor_entry_write(actor_id).await {
            actor_entry.put_data(data);
        }
    }

    pub async fn get_data<D: Any + Clone>(&self, actor_id: ActorID) -> Option<D> {
        self.actor_entry_read(actor_id)
            .await
            .and_then(|actor_entry| actor_entry.get_data().cloned())
    }

    pub async fn take_data<D: Any>(&self, actor_id: ActorID) -> Option<D> {
        self.actor_entry_write(actor_id)
            .await
            .and_then(|mut actor_entry| actor_entry.take_data())
    }

    pub fn all_actors(&self) -> impl Stream<Item = ActorID> + '_ {
        stream::iter(&self.0.actor_entries[..])
            .filter_map(|slot| async move { slot.read().await.running_actor_id() })
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
    actor_entries: Box<[RwLock<ActorEntry>]>,
    exit_handler: Arc<dyn ExitHandler>,
}
