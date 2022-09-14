use std::collections::HashSet;

use crate::actor_id::ActorID;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct Watches {
	pub trap_exit: bool,
	pub links: HashSet<ActorID>,
}

impl<M> Backend<M> {
	pub(super) async fn notify_linked_actors(&mut self, exit_reason: Arc<ExitReason>) {
		for linked in std::mem::replace(&mut self.watches, Default::default()).links.drain() {
			self.send_sys_msg(linked, SysMsg::Exit(self.actor_id, exit_reason.to_owned()))
				.await;
		}
	}

	pub(super) async fn handle_call_link(&mut self, link_to: ActorID) -> Result<(), ExitReason> {
		if self.watches.links.insert(link_to) {
			if !self.send_sys_msg(link_to, SysMsg::Link(self.actor_id)).await {
				let _ =
					self.sys_msg_tx.send(SysMsg::Exit(link_to, Arc::new(ExitReason::NoProcess)));
			}
		}
		Ok(())
	}

	pub(super) async fn handle_call_unlink(
		&mut self,
		unlink_from: ActorID,
	) -> Result<(), ExitReason> {
		if self.watches.links.remove(&unlink_from) {
			self.send_sys_msg(unlink_from, SysMsg::Unlink(self.actor_id)).await;
		}
		Ok(())
	}

	pub(super) async fn handle_sys_msg_exit(
		&mut self,
		terminated: ActorID,
		exit_reason: Arc<ExitReason>,
	) -> Result<(), ExitReason> {
		if self.watches.links.remove(&terminated) {
			if self.watches.trap_exit {
				let signal = Signal::Exited(terminated, exit_reason);
				self.signals_w
					.send(signal)
					.await
					.map_err(|_| ExitReason::InboxFull("signals"))?;
				Ok(())
			} else {
				Err(ExitReason::Exited(terminated, exit_reason))
			}
		} else {
			Ok(())
		}
	}
	pub(super) async fn handle_sys_msg_link(&mut self, link_to: ActorID) -> Result<(), ExitReason> {
		self.watches.links.insert(link_to);
		Ok(())
	}
	pub(super) async fn handle_sys_msg_unlink(
		&mut self,
		unlink_from: ActorID,
	) -> Result<(), ExitReason> {
		self.watches.links.remove(&unlink_from);
		Ok(())
	}
}
