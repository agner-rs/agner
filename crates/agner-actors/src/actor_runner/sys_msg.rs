use std::sync::Arc;

use crate::actor_id::ActorID;
use crate::exit_reason::ExitReason;

use super::Backend;

#[derive(Debug)]
pub enum SysMsg {
	Link(ActorID),
	Unlink(ActorID),
	Exit(ActorID, Arc<ExitReason>),
}

impl<M> Backend<M> {
	pub(super) async fn send_sys_msg(&self, to: ActorID, sys_msg: SysMsg) -> bool {
		// if the system is still alive,
		if let Some(system) = self.system_opt.rc_upgrade() {
			// if there is no existing link,
			let sent = system
				.actor_entry_read(to, |entry| entry.sys_msg_tx.send(sys_msg).ok())
				.await
				.flatten()
				.is_some();
			sent
		} else {
			false
		}
	}
}
