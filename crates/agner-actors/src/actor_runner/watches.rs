use std::collections::HashSet;

use tokio::sync::oneshot;

use crate::actor_id::ActorID;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct Watches {
    pub trap_exit: bool,
    pub links: HashSet<ActorID>,
    pub waits: Vec<oneshot::Sender<ExitReason>>,
}

impl<M> Backend<M> {
    pub(super) async fn notify_linked_actors(&mut self, exit_reason: ExitReason) {
        for linked in std::mem::replace(&mut self.watches.links, Default::default()).drain() {
            if matches!(exit_reason, ExitReason::Normal) {
                self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
            } else {
                log::trace!("[{}] notifying linked actor: {}", self.actor_id, linked);
                self.send_sys_msg(linked, SysMsg::SigExit(self.actor_id, exit_reason.to_owned()))
                    .await;
            }
        }
    }
    pub(super) fn notify_waiting_chans(&mut self, exit_reason: ExitReason) {
        for (idx, report_to) in std::mem::replace(&mut self.watches.waits, Default::default())
            .into_iter()
            .enumerate()
            .filter(|(_idx, c)| !c.is_closed())
        {
            log::trace!("[{}] notifying waiting chan #{}", self.actor_id, idx);
            let _ = report_to.send(exit_reason.to_owned());
        }
    }

    pub(super) async fn do_link(&mut self, link_to: ActorID) {
        if self.watches.links.insert(link_to) {
            log::trace!("[{}] linking to {}", self.actor_id, link_to);

            if !self.send_sys_msg(link_to, SysMsg::Link(self.actor_id)).await {
                let _ = self.sys_msg_tx.send(SysMsg::SigExit(link_to, ExitReason::NoProcess));
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

    pub(super) async fn handle_sys_msg_sig_exit(
        &mut self,
        receiver_id: ActorID,
        exit_reason: ExitReason,
    ) -> Result<(), ExitReason> {
        if receiver_id == self.actor_id || self.watches.links.remove(&receiver_id) {
            log::trace!(
                "[{}] Received SigExit({}, ..) [trap-exit: {}]",
                self.actor_id,
                receiver_id,
                self.watches.trap_exit
            );

            match (
                self.watches.trap_exit,
                receiver_id == self.actor_id,
                matches!(exit_reason, ExitReason::Kill),
            ) {
                (_, true, true) => Err(ExitReason::Kill),

                (false, true, _) => Err(exit_reason),
                (false, false, _) => Err(ExitReason::Exited(receiver_id, exit_reason.into())),

                (true, _, _) => {
                    let signal = Signal::Exit(receiver_id, exit_reason);
                    self.signals_w
                        .send(signal)
                        .await
                        .map_err(|_| ExitReason::InboxFull("signals"))?;
                    Ok(())
                },
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
        report_to: oneshot::Sender<ExitReason>,
    ) -> Result<(), ExitReason> {
        if let Some((idx, to_replace)) =
            self.watches.waits.iter_mut().enumerate().find(|(_idx, tx)| tx.is_closed())
        {
            log::trace!("[{}] adding 'wait' [replace #{}]", self.actor_id, idx);
            *to_replace = report_to;
        } else {
            log::trace!("[{}] adding 'wait' [append #{}]", self.actor_id, self.watches.waits.len());
            self.watches.waits.push(report_to);
        }
        Ok(())
    }
}
