use std::sync::Arc;

use tokio::sync::oneshot;

use crate::actor_id::ActorID;
use crate::exit_reason::ExitReason;

use super::Backend;

#[derive(Debug)]
pub enum SysMsg {
    Link(ActorID),
    Unlink(ActorID),
    SigExit(ActorID, Arc<ExitReason>),
    Wait(oneshot::Sender<Arc<ExitReason>>),
}

impl<M> Backend<M> {
    pub(super) async fn send_sys_msg(&self, to: ActorID, sys_msg: SysMsg) -> bool {
        if let Some(system) = self.system_opt.rc_upgrade() {
            system.send_sys_msg(to, sys_msg).await
        } else {
            false
        }
    }
}
