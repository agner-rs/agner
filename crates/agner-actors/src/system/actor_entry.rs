use std::any::Any;

use tokio::sync::mpsc;

use crate::actor_id::ActorID;
use crate::actor_runner::sys_msg::SysMsg;
use crate::system::actor_id_pool::ActorIDLease;
use crate::system::System;

#[derive(Debug)]
pub(crate) struct ActorEntry {
    pub actor_id_lease: ActorIDLease,
    pub sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    pub messages_tx: Box<dyn Any + Send + Sync + 'static>,
}

impl System {
    pub(crate) async fn actor_entry_put(&self, entry: ActorEntry) {
        if entry.actor_id_lease.system() != self.0.system_id {
            panic!(
                "attempt to insert an entry with a foreign actor-id [expected: {}; actual: {}]",
                self.0.system_id,
                entry.actor_id_lease.system()
            )
        }

        let slot_idx = entry.actor_id_lease.actor();
        let mut slot = self.0.actor_entries[slot_idx].write().await;
        let should_be_none = slot.replace(entry);
        assert!(should_be_none.is_none());
    }
    pub(crate) async fn actor_entry_remove(&self, actor_id: ActorID) {
        if actor_id.system() != self.0.system_id {
            panic!(
                "attempt to remove an entry with a foreign actor-id [expected: {}; actual: {}]",
                self.0.system_id,
                actor_id.system()
            )
        }

        let slot_idx = actor_id.actor();
        let mut slot = self.0.actor_entries[slot_idx].write().await;
        if slot.as_ref().map(|e| *e.actor_id_lease) == Some(actor_id) {
            let should_be_some = slot.take();
            assert!(should_be_some.is_some())
        }
    }
    pub(crate) async fn actor_entry_read<F, Out>(&self, actor_id: ActorID, read: F) -> Option<Out>
    where
        F: FnOnce(&ActorEntry) -> Out,
    {
        if actor_id.system() != self.0.system_id {
            return None
        }

        let slot_idx = actor_id.actor();
        let slot = self.0.actor_entries.get(slot_idx)?.read().await;
        slot.as_ref().filter(|e| *e.actor_id_lease == actor_id).map(read)
    }
}
