use std::fmt;
use std::future::Future;
use std::pin::Pin;

use crate::actor_id::ActorID;
use crate::exit::Exit;

pub enum CallMsg<M> {
    Exit(Exit),
    Link(ActorID),
    Unlink(ActorID),
    TrapExit(bool),
    SpawnJob(Pin<Box<dyn Future<Output = Option<M>> + Send + Sync + 'static>>),
}

impl<M> fmt::Debug for CallMsg<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exit(reason) => f.debug_tuple("Exit").field(reason).finish(),
            Self::Link(actor_id) => f.debug_tuple("Link").field(actor_id).finish(),
            Self::Unlink(actor_id) => f.debug_tuple("Unlink").field(actor_id).finish(),
            Self::TrapExit(trap_exit) => f.debug_tuple("TrapExit").field(trap_exit).finish(),
            Self::SpawnJob { .. } => f.debug_tuple("SpawnJob").finish(),
        }
    }
}
