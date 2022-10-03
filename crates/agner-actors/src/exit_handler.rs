use std::fmt;

use agner_utils::std_error_pp::StdErrorPP;

use crate::actor_id::ActorID;
use crate::exit::{Exit, ExitStandard};

pub trait ExitHandler: fmt::Debug + Send + Sync + 'static {
    fn on_actor_exit(&self, actor_id: ActorID, exit: Exit);
}

#[derive(Debug, Clone, Copy)]
pub struct LogExitHandler;

#[derive(Debug, Clone, Copy)]
pub struct NoopExitHandler;

impl ExitHandler for LogExitHandler {
    fn on_actor_exit(&self, actor_id: ActorID, exit: Exit) {
        match exit {
            Exit::Standard(ExitStandard::Normal | ExitStandard::Shutdown(None)) => (),
            Exit::Standard(ExitStandard::Linked(offender, reason)) => {
                log::warn!("[{}] linked {} exited: {}", actor_id, offender, reason.pp());
            },
            failure => {
                log::error!("[{}] {}", actor_id, failure.pp())
            },
        }
    }
}

impl ExitHandler for NoopExitHandler {
    fn on_actor_exit(&self, _actor_id: ActorID, _exit: Exit) {}
}
