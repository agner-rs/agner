use std::ops::{Deref, DerefMut};

use agner_utils::std_error_pp::StdErrorPP;

use super::*;

impl System {
    pub(crate) async fn actor_entry_put(&self, entry: ActorEntry) {
        let actor_id =
            entry.running_actor_id().expect("Attempt to insert a non-running actor-entry");
        assert_eq!(
            actor_id.system(), self.0.system_id,
            "attempt to insert an entry with a foreign actor-id [this-system-id: {}; entry-system-id: {}]",
            self.0.system_id, actor_id.system());

        let slot_idx = actor_id.actor();
        let mut slot = self.0.actor_entries[slot_idx].write().await;
        let should_be_none = std::mem::replace(&mut *slot, entry);
        assert!(should_be_none.running_actor_id().is_none());
    }

    pub(crate) fn actor_entry_slot(&self, actor_id: ActorID) -> &RwLock<ActorEntry> {
        if actor_id.system() != self.0.system_id {
            panic!(
                "attempt to terminate an entry with a foreign actor-id [this-system-id: {}; entry-system-id: {}]",
                self.0.system_id,
                actor_id.system()
            )
        }

        let slot_idx = actor_id.actor();

        &self.0.actor_entries[slot_idx]
    }

    pub(crate) async fn actor_entry_read(
        &self,
        actor_id: ActorID,
    ) -> Option<impl Deref<Target = ActorEntry> + '_> {
        let locked = self.actor_entry_slot(actor_id).read().await;
        if locked.running_or_terminated_actor_id() == Some(actor_id) {
            Some(locked)
        } else {
            None
        }
    }
    pub(crate) async fn actor_entry_write<'a>(
        &'a self,
        actor_id: ActorID,
    ) -> Option<impl DerefMut<Target = ActorEntry> + 'a> {
        let locked = self.actor_entry_slot(actor_id).write().await;
        if locked.running_or_terminated_actor_id() == Some(actor_id) {
            Some(locked)
        } else {
            None
        }
    }

    pub(crate) async fn actor_entry_terminate(&self, actor_id: ActorID, exit_reason: Exit) {
        if let Err(reason) = self
            .actor_entry_write(actor_id)
            .await
            .map(|mut ae| ae.terminate(actor_id, exit_reason))
            .transpose()
        {
            log::error!("Failed to terminate ActorEntry: {}", reason.as_ref().pp());
        }
    }
}
