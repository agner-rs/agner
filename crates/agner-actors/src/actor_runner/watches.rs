use std::collections::HashSet;

use tokio::sync::oneshot;

use crate::actor_id::ActorID;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct Watches {
    pub trap_exit: bool,
    pub links: HashSet<ActorID>,
    pub waits: Vec<oneshot::Sender<Arc<ExitReason>>>,
}

impl<M> Backend<M> {
    pub(super) async fn notify_linked_actors(&mut self, exit_reason: Arc<ExitReason>) {
        for linked in std::mem::replace(&mut self.watches.links, Default::default()).drain() {
            if matches!(*exit_reason, ExitReason::Normal) {
                self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
            } else {
                log::trace!("[{}] notifying linked actor: {}", self.actor_id, linked);
                self.send_sys_msg(linked, SysMsg::Exited(self.actor_id, exit_reason.to_owned()))
                    .await;
            }
        }
    }
    pub(super) fn notify_waiting_chans(&mut self, exit_reason: Arc<ExitReason>) {
        for report_to in std::mem::replace(&mut self.watches.waits, Default::default())
            .drain(..)
            .filter(|c| !c.is_closed())
        {
            log::trace!("[{}] notifying waiting chan", self.actor_id);
            let _ = report_to.send(exit_reason.to_owned());
        }
    }

    pub(super) async fn do_link(&mut self, link_to: ActorID) {
        if self.watches.links.insert(link_to) {
            log::trace!("[{}] linking to {}", self.actor_id, link_to);

            if !self.send_sys_msg(link_to, SysMsg::Link(self.actor_id)).await {
                let _ =
                    self.sys_msg_tx.send(SysMsg::Exited(link_to, Arc::new(ExitReason::NoProcess)));
            }
        }
    }
    pub(super) async fn do_unlink(&mut self, unlink_from: ActorID) {
        if self.watches.links.remove(&unlink_from) {
            log::trace!("[{}] unlinking from {}", self.actor_id, unlink_from);

            self.send_sys_msg(unlink_from, SysMsg::Unlink(self.actor_id)).await;
        }
    }

    pub(super) fn handle_set_trap_exit(&mut self, trap_exit: bool) -> Result<(), ExitReason> {
        if self.watches.trap_exit != trap_exit {
            log::trace!("[{}] trap_exit = {}", self.actor_id, trap_exit);
            self.watches.trap_exit = trap_exit;
        }
        Ok(())
    }

    pub(super) async fn handle_call_link(&mut self, link_to: ActorID) -> Result<(), ExitReason> {
        self.do_link(link_to).await;
        Ok(())
    }

    pub(super) async fn handle_call_unlink(
        &mut self,
        unlink_from: ActorID,
    ) -> Result<(), ExitReason> {
        self.do_unlink(unlink_from).await;
        Ok(())
    }

    pub(super) async fn handle_sys_msg_exit(
        &mut self,
        terminated: ActorID,
        exit_reason: Arc<ExitReason>,
    ) -> Result<(), ExitReason> {
        if self.watches.links.remove(&terminated) {
            log::trace!(
                "[{}] Received Signal::Exited({}, ..) [trap-exit: {}]",
                self.actor_id,
                terminated,
                self.watches.trap_exit
            );
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

    pub(super) fn handle_sys_msg_wait(
        &mut self,
        report_to: oneshot::Sender<Arc<ExitReason>>,
    ) -> Result<(), ExitReason> {
        if let Some(to_replace) = self.watches.waits.iter_mut().find(|tx| tx.is_closed()) {
            log::trace!("adding 'wait' [replace]");
            *to_replace = report_to;
        } else {
            log::trace!("adding 'wait' [append]");
            self.watches.waits.push(report_to);
        }
        Ok(())
    }
}
