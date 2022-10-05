use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::exit_handler::ExitHandler;

const DEFAULT_MSG_INBOX_SIZE: usize = 1024;
const DEFAULT_SIG_INBOX_SIZE: usize = 16;

/// Options with which an actor will be spawned.
///
/// It is possible to specify:
/// - the set of [actor-ids](crate::actor_id::ActorID) the newly spawned actor will be immediately
///   linked to;
/// - the sizes for msg-inbox and signal-inbox;
/// - [exit-handler](crate::exit_handler::ExitHandler);
/// - a "bag" of arbitrary properties (identified by their types).
#[derive(Debug)]
pub struct SpawnOpts {
    links: HashSet<ActorID>,
    msg_inbox_size: usize,
    sig_inbox_size: usize,
    exit_handler: Option<Arc<dyn ExitHandler>>,
    data: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
}

impl Default for SpawnOpts {
    fn default() -> Self {
        Self {
            links: Default::default(),
            msg_inbox_size: DEFAULT_MSG_INBOX_SIZE,
            sig_inbox_size: DEFAULT_SIG_INBOX_SIZE,
            exit_handler: None,
            data: Default::default(),
        }
    }
}

impl SpawnOpts {
    /// create new [SpawnOpts](SpawnOpts)
    pub fn new() -> Self {
        Default::default()
    }
}

impl SpawnOpts {
    /// add a linked actor
    pub fn with_link(mut self, with: ActorID) -> Self {
        self.links.insert(with);
        self
    }
    /// iterator of linked actors
    pub fn links(&self) -> impl Iterator<Item = ActorID> + '_ {
        self.links.iter().copied()
    }
}

impl SpawnOpts {
    /// specify the capacity limit for msg-inbox
    pub fn with_msg_inbox_size(mut self, sz: usize) -> Self {
        self.msg_inbox_size = sz;
        self
    }

    /// the capacity limit for msg-inbox
    pub fn msg_inbox_size(&self) -> usize {
        self.msg_inbox_size
    }
}

impl SpawnOpts {
    /// specify the capacity limit for signal-inbox
    pub fn with_sig_inbox_size(mut self, sz: usize) -> Self {
        self.sig_inbox_size = sz;
        self
    }

    /// the capacity limit for signal-inbox
    pub fn sig_inbox_size(&self) -> usize {
        self.sig_inbox_size
    }
}

impl SpawnOpts {
    /// add arbitrary data into the [`Context`](crate::context::Context)
    pub fn with_data<D>(mut self, data: D) -> Self
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = data.type_id();
        let boxed = Box::new(data);
        self.data.insert(type_id, boxed);
        self
    }

    pub(crate) fn take_data(&mut self) -> HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>> {
        std::mem::take(&mut self.data)
    }
}

impl SpawnOpts {
    /// Specify the [exit-handler](crate::exit_handler::ExitHandler) for the spawned actor
    pub fn with_exit_handler(mut self, exit_handler: Arc<dyn ExitHandler>) -> Self {
        let _ = self.exit_handler.replace(exit_handler);
        self
    }
    pub(crate) fn take_exit_handler(&mut self) -> Option<Arc<dyn ExitHandler>> {
        self.exit_handler.take()
    }
}
