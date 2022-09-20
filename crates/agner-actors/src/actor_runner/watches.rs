use std::collections::HashSet;

use crate::actor_id::ActorID;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct Watches {
    pub trap_exit: bool,
    pub links: HashSet<ActorID>,
}

impl<M> Backend<M> {
    pub(super) async fn notify_linked_actors(&mut self, exit_reason: Exit) {
        for linked in std::mem::replace(&mut self.watches.links, Default::default()).drain() {
            if exit_reason.is_normal() {
                self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
            } else {
                log::trace!("[{}] notifying linked actor: {}", self.actor_id, linked);
                self.send_sys_msg(linked, SysMsg::SigExit(self.actor_id, exit_reason.to_owned()))
                    .await;
            }
        }
    }

    pub(super) async fn do_link(&mut self, link_to: ActorID) {
        if self.watches.links.insert(link_to) {
            log::trace!("[{}] linking to {}", self.actor_id, link_to);

            if !self.send_sys_msg(link_to, SysMsg::Link(self.actor_id)).await {
                let _ = self.sys_msg_tx.send(SysMsg::SigExit(link_to, Exit::no_actor()));
            }
        }
    }
    pub(super) async fn do_unlink(&mut self, unlink_from: ActorID) {
        if self.watches.links.remove(&unlink_from) {
            log::trace!("[{}] unlinking from {}", self.actor_id, unlink_from);

            self.send_sys_msg(unlink_from, SysMsg::Unlink(self.actor_id)).await;
        }
    }

    pub(super) fn handle_set_trap_exit(&mut self, trap_exit: bool) -> Result<(), Exit> {
        if self.watches.trap_exit != trap_exit {
            log::trace!("[{}] trap_exit = {}", self.actor_id, trap_exit);
            self.watches.trap_exit = trap_exit;
        }
        Ok(())
    }

    pub(super) async fn handle_call_link(&mut self, link_to: ActorID) -> Result<(), Exit> {
        self.do_link(link_to).await;
        Ok(())
    }

    pub(super) async fn handle_call_unlink(&mut self, unlink_from: ActorID) -> Result<(), Exit> {
        self.do_unlink(unlink_from).await;
        Ok(())
    }

    pub(super) async fn handle_sys_msg_sig_exit(
        &mut self,
        receiver_id: ActorID,
        exit_reason: Exit,
    ) -> Result<(), Exit> {
        if receiver_id == self.actor_id || self.watches.links.remove(&receiver_id) {
            log::trace!(
                "[{}] Received SigExit({}, ..) [trap-exit: {}]",
                self.actor_id,
                receiver_id,
                self.watches.trap_exit
            );

            match (self.watches.trap_exit, receiver_id == self.actor_id, exit_reason.is_kill()) {
                (_, true, true) => Err(Exit::kill()),

                (false, true, _) => Err(exit_reason),
                (false, false, _) => Err(Exit::exited(receiver_id, exit_reason)),

                (true, _, _) => {
                    let signal = Signal::Exit(receiver_id, exit_reason);
                    self.signals_w
                        .send(signal)
                        .await
                        .map_err(|_| BackendFailure::InboxFull("signals"))?;
                    Ok(())
                },
            }
        } else {
            Ok(())
        }
    }
    pub(super) async fn handle_sys_msg_link(&mut self, link_to: ActorID) -> Result<(), Exit> {
        self.watches.links.insert(link_to);
        Ok(())
    }
    pub(super) async fn handle_sys_msg_unlink(&mut self, unlink_from: ActorID) -> Result<(), Exit> {
        self.watches.links.remove(&unlink_from);
        Ok(())
    }
}
