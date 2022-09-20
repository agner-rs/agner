use std::any::Any;
use std::error::Error as StdError;
use std::time::Instant;

use futures::future::Either;
use tokio::sync::{mpsc, oneshot};

use crate::actor_id::ActorID;
use crate::actor_runner::sys_msg::SysMsg;
use crate::exit::Exit;

use super::actor_id_pool::ActorIDLease;

#[derive(Debug)]
pub struct ActorEntry(Entry);

#[derive(Debug)]
enum Entry {
    Vacant(Vacant),
    Occupied(Occupied),
}

#[derive(Debug)]
struct Occupied {
    actor_id_lease: ActorIDLease,
    messages_tx: Box<dyn Any + Send + Sync + 'static>,
    sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    watches: Vec<oneshot::Sender<Exit>>,
}

type Vacant = Either<Terminated, Vec<oneshot::Sender<Exit>>>;

#[derive(Debug)]
struct Terminated {
    actor_id: ActorID,
    exit: Exit,
    at: Instant,
}

impl Default for ActorEntry {
    fn default() -> Self {
        Self(Entry::Vacant(Either::Right(Default::default())))
    }
}

impl ActorEntry {
    pub fn running_actor_id(&self) -> Option<ActorID> {
        self.occupied().map(|oe| *oe.actor_id_lease)
    }

    pub fn running_or_terminated_actor_id(&self) -> Option<ActorID> {
        match &self.0 {
            Entry::Occupied(occupied) => Some(*occupied.actor_id_lease),
            Entry::Vacant(Either::Left(terminated)) => Some(terminated.actor_id),
            Entry::Vacant(Either::Right(_)) => None,
        }
    }

    pub fn messages_tx<M>(&self) -> Option<&mpsc::UnboundedSender<M>>
    where
        M: Send + Sync + 'static,
    {
        self.occupied().and_then(|oe| oe.messages_tx.downcast_ref())
    }
    pub fn sys_msg_tx(&self) -> Option<&mpsc::UnboundedSender<SysMsg>> {
        self.occupied().map(|oe| &oe.sys_msg_tx)
    }
}

impl ActorEntry {
    pub fn new<Message>(
        actor_id_lease: ActorIDLease,
        messages_tx: mpsc::UnboundedSender<Message>,
        sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    ) -> Self
    where
        Message: Send + Sync + 'static,
    {
        let occupied = Occupied {
            actor_id_lease,
            messages_tx: Box::new(messages_tx),
            sys_msg_tx,
            watches: Default::default(),
        };
        let entry = Entry::Occupied(occupied);
        Self(entry)
    }

    pub fn add_watch(&mut self, watch: oneshot::Sender<Exit>) {
        fn replace_or_append(
            actor_id: ActorID,
            watches: &mut Vec<oneshot::Sender<Exit>>,
            watch: oneshot::Sender<Exit>,
        ) {
            let maybe_replace = watches.iter_mut().enumerate().find(|(_idx, tx)| tx.is_closed());
            if let Some((idx, to_replace)) = maybe_replace {
                log::trace!("[{}] adding 'wait' [replace #{}]", actor_id, idx);
                *to_replace = watch;
            } else {
                log::trace!("[{}] adding 'wait' [append #{}]", actor_id, watches.len());
                watches.push(watch);
            }
        }
        match &mut self.0 {
            Entry::Vacant(Either::Right(_wathces)) => unimplemented!(
                "There is no way the control gets here before the entry is initialized"
            ),
            Entry::Vacant(Either::Left(Terminated { exit, .. })) => {
                let _ = watch.send(exit.to_owned());
            },
            Entry::Occupied(occupied) => {
                replace_or_append(*occupied.actor_id_lease, &mut occupied.watches, watch);
            },
        }
    }

    pub fn terminate(
        &mut self,
        actor_id: ActorID,
        exit_reason: Exit,
    ) -> Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        if self.running_actor_id() != Some(actor_id) {
            Err("this entry does not have a running entry with the specified actor_id")?
        }

        let to_terminate = std::mem::replace(
            &mut self.0,
            Entry::Vacant(Either::Left(Terminated {
                actor_id,
                exit: exit_reason.to_owned(),
                at: Instant::now(),
            })),
        );

        if let Entry::Occupied(Occupied { mut watches, .. }) = to_terminate {
            watches.drain(..).for_each(|tx| {
                let _ = tx.send(exit_reason.to_owned());
            });
        }
        Ok(())
    }
}

impl ActorEntry {
    fn occupied(&self) -> Option<&Occupied> {
        if let Entry::Occupied(occupied) = &self.0 {
            Some(occupied)
        } else {
            None
        }
    }
}
