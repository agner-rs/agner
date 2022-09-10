use super::*;

#[derive(Debug)]
pub(crate) struct EntryCleanupGuard {
	pub(super) system: SystemOne,
	pub(super) actor_id: ActorID,
}

impl Drop for EntryCleanupGuard {
	fn drop(&mut self) {
		let (_system_id, actor_idx, _serial_id) = self.actor_id.into();
		if self.system.entry_write(self.actor_id, |entry| entry.take()).is_some() {
			self.system
				.0
				.ids_pool
				.lock()
				.expect("Failed to lock `ids_pool`")
				.1
				.push_front(actor_idx);
		}
	}
}
