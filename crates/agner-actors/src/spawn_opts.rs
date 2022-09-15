use std::collections::HashSet;

use crate::actor_id::ActorID;
use crate::init_ack::InitAckTx;

const DEFAULT_MSG_INBOX_SIZE: usize = 1024;
const DEFAULT_SIG_INBOX_SIZE: usize = 16;

#[derive(Debug)]
pub struct SpawnOpts {
    links: HashSet<ActorID>,
    msg_inbox_size: usize,
    sig_inbox_size: usize,
    init_ack: Option<InitAckTx>,
}

impl Default for SpawnOpts {
    fn default() -> Self {
        Self {
            links: Default::default(),
            msg_inbox_size: DEFAULT_MSG_INBOX_SIZE,
            sig_inbox_size: DEFAULT_SIG_INBOX_SIZE,
            init_ack: None,
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
    pub fn links<'a>(&'a self) -> impl Iterator<Item = ActorID> + 'a {
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

    pub fn with_init_ack(mut self, init_ack_tx: InitAckTx) -> Self {
        let _ = self.init_ack.replace(init_ack_tx);
        self
    }
    pub fn take_init_ack(&mut self) -> Option<InitAckTx> {
        self.init_ack.take()
    }
}
