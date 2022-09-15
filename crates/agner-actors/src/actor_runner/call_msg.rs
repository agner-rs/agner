use std::fmt;
use std::future::Future;
use std::pin::Pin;

use futures::Stream;

use crate::actor_id::ActorID;
use crate::exit_reason::ExitReason;

pub enum CallMsg<M> {
    Exit(ExitReason),
    Link(ActorID),
    Unlink(ActorID),
    TrapExit(bool),
    FutureToInbox(Pin<Box<dyn Future<Output = M> + Send + Sync + 'static>>),
}

impl<M> fmt::Debug for CallMsg<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exit(reason) => f.debug_tuple("Exit").field(reason).finish(),
            Self::Link(actor_id) => f.debug_tuple("Link").field(actor_id).finish(),
            Self::Unlink(actor_id) => f.debug_tuple("Unlink").field(actor_id).finish(),
            Self::TrapExit(trap_exit) => f.debug_tuple("TrapExit").field(trap_exit).finish(),
            Self::FutureToInbox { .. } => f.debug_tuple("FutureToInbox").finish(),
        }
    }
}
