use std::collections::HashSet;

use crate::actor_id::ActorID;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct Watches {
    pub trap_exit: bool,
    pub links: HashSet<ActorID>,
}

impl<M> Backend<M> {
    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        exit_reason = display(exit_reason.pp())
    ))]
    pub(super) async fn notify_linked_actors(&mut self, exit_reason: Exit) {
        for linked in std::mem::take(&mut self.watches.links).drain() {
            if exit_reason.is_normal() {
                self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
            } else {
                tracing::trace!("notifying linked actor: {}", linked);
                self.send_sys_msg(linked, SysMsg::SigExit(self.actor_id, exit_reason.to_owned()))
                    .await;
            }
        }
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        link_to = display(link_to))
    )]
    pub(super) async fn do_link(&mut self, link_to: ActorID) {
        if self.watches.links.insert(link_to) {
            tracing::trace!("linking to {}", link_to);

            if !self.send_sys_msg(link_to, SysMsg::Link(self.actor_id)).await {
                let _ = self.sys_msg_tx.send(SysMsg::SigExit(link_to, Exit::no_actor()));
            }
        }
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        unlink_from = display(unlink_from)
    ))]
    pub(super) async fn do_unlink(&mut self, unlink_from: ActorID) {
        if self.watches.links.remove(&unlink_from) {
            tracing::trace!("[{}] unlinking from {}", self.actor_id, unlink_from);

            self.send_sys_msg(unlink_from, SysMsg::Unlink(self.actor_id)).await;
        }
    }

    #[tracing::instrument(skip(self), fields(
        actor_id = display(self.actor_id)
    ))]
    pub(super) fn handle_set_trap_exit(&mut self, trap_exit: bool) -> Result<(), Exit> {
        if self.watches.trap_exit != trap_exit {
            tracing::trace!("trap_exit = {}", trap_exit);
            self.watches.trap_exit = trap_exit;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        link_to = display(link_to)
    ))]
    pub(super) async fn handle_call_link(&mut self, link_to: ActorID) -> Result<(), Exit> {
        self.do_link(link_to).await;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        unlink_from = display(unlink_from)
    ))]
    pub(super) async fn handle_call_unlink(&mut self, unlink_from: ActorID) -> Result<(), Exit> {
        self.do_unlink(unlink_from).await;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        receiver_id = display(receiver_id),
        exit_reason = display(exit_reason.pp())
    ))]
    pub(super) async fn handle_sys_msg_sig_exit(
        &mut self,
        receiver_id: ActorID,
        exit_reason: Exit,
    ) -> Result<(), Exit> {
        if receiver_id == self.actor_id || self.watches.links.remove(&receiver_id) {
            tracing::trace!(
                "[{}] Received SigExit({}, ..) [trap-exit: {}]",
                self.actor_id,
                receiver_id,
                self.watches.trap_exit
            );

            match (self.watches.trap_exit, receiver_id == self.actor_id, exit_reason.is_kill()) {
                (_, true, true) => Err(Exit::kill()),

                (false, true, _) => Err(exit_reason),
                (false, false, _) => Err(Exit::linked(receiver_id, exit_reason)),

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

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        link_to = display(link_to)
    ))]
    pub(super) async fn handle_sys_msg_link(&mut self, link_to: ActorID) -> Result<(), Exit> {
        self.watches.links.insert(link_to);
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        unlink_from = display(unlink_from)
    ))]
    pub(super) async fn handle_sys_msg_unlink(&mut self, unlink_from: ActorID) -> Result<(), Exit> {
        self.watches.links.remove(&unlink_from);
        Ok(())
    }
}
