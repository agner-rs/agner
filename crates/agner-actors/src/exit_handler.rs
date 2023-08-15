use std::fmt;

use agner_utils::std_error_pp::StdErrorPP;

use crate::actor_id::ActorID;
use crate::exit::{Exit, Shutdown, WellKnown};

/// `ExitHandler` is an entity that is notified when an actor exits.
///
/// Each actor has an `ExitHandler` associated with it.
/// It is possible to specify an exit-handler for an actor via
/// [`SpawnOpts::with_exit_handler`](crate::spawn_opts::SpawnOpts::with_exit_handler).
pub trait ExitHandler: fmt::Debug + Send + Sync + 'static {
    fn on_actor_exit(&self, actor_id: ActorID, exit: Exit);
}

/// An [`ExitHandler`](crate::exit_handler::ExitHandler) that will log abnormal exits.
#[derive(Debug, Clone, Copy)]
pub struct LogExitHandler;

/// An [`ExitHandler`](crate::exit_handler::ExitHandler) that will print abnormal exits into stderr.
#[derive(Debug, Clone, Copy)]
pub struct StderrExitHandler;

/// A no-op [`ExitHandler`](crate::exit_handler::ExitHandler), i.e. it does nothing.
#[derive(Debug, Clone, Copy)]
pub struct NoopExitHandler;

impl ExitHandler for LogExitHandler {
    fn on_actor_exit(&self, actor_id: ActorID, exit: Exit) {
        match exit {
            Exit::Standard(WellKnown::Normal | WellKnown::Shutdown(Shutdown(None))) => (),
            Exit::Standard(WellKnown::Linked(offender, reason)) => {
                tracing::warn!("[{}] linked {} exited: {}", actor_id, offender, reason.pp());
            },
            failure => {
                tracing::error!("[{}] {}", actor_id, failure.pp());
            },
        }
    }
}

impl ExitHandler for StderrExitHandler {
    fn on_actor_exit(&self, actor_id: ActorID, exit: Exit) {
        match exit {
            Exit::Standard(WellKnown::Normal | WellKnown::Shutdown(Shutdown(None))) => (),
            Exit::Standard(WellKnown::Linked(offender, reason)) => {
                eprintln!("[{}] linked {} exited: {}", actor_id, offender, reason.pp());
            },
            failure => {
                eprintln!("[{}] {}", actor_id, failure.pp());
            },
        }
    }
}

impl ExitHandler for NoopExitHandler {
    fn on_actor_exit(&self, _actor_id: ActorID, _exit: Exit) {}
}
