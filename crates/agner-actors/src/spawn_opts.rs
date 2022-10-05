use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::exit_handler::ExitHandler;

const DEFAULT_MSG_INBOX_SIZE: usize = 1024;
const DEFAULT_SIG_INBOX_SIZE: usize = 16;

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
    pub fn new() -> Self {
        Default::default()
    }
}

impl SpawnOpts {
    pub fn with_link(mut self, with: ActorID) -> Self {
        self.links.insert(with);
        self
    }
    pub fn links(&self) -> impl Iterator<Item = ActorID> + '_ {
        self.links.iter().copied()
    }
}

impl SpawnOpts {
    pub fn with_msg_inbox_size(mut self, sz: usize) -> Self {
        self.msg_inbox_size = sz;
        self
    }
    pub fn msg_inbox_size(&self) -> usize {
        self.msg_inbox_size
    }

    pub fn with_sig_inbox_size(mut self, sz: usize) -> Self {
        self.sig_inbox_size = sz;
        self
    }
    pub fn sig_inbox_size(&self) -> usize {
        self.sig_inbox_size
    }

    pub fn with_data<D>(mut self, data: D) -> Self
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = data.type_id();
        let boxed = Box::new(data);
        self.data.insert(type_id, boxed);
        self
    }

    pub fn take_data(&mut self) -> HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>> {
        std::mem::take(&mut self.data)
    }

    pub fn with_exit_handler(mut self, exit_handler: Arc<dyn ExitHandler>) -> Self {
        let _ = self.exit_handler.replace(exit_handler);
        self
    }
    pub fn take_exit_handler(&mut self) -> Option<Arc<dyn ExitHandler>> {
        self.exit_handler.take()
    }
}
