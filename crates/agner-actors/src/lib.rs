mod actor;
mod actor_id;
mod actor_runner;
mod context;
mod exit_reason;
mod init_ack;
mod spawn_opts;
mod system;
mod system_config;

mod exports {
    pub use crate::actor::{Actor, IntoExitReason};
    pub use crate::actor_id::ActorID;
    pub use crate::actor_runner::ActorInfo;
    pub use crate::context::{Context, Event, Signal};
    pub use crate::exit_reason::ExitReason;
    pub use crate::init_ack::{new as new_init_ack, InitAckRx, InitAckTx};
    pub use crate::spawn_opts::SpawnOpts;
    pub use crate::system::{SysChannelError, SysSpawnError, System};
    pub use crate::system_config::SystemConfig;
}
mod imports {
    use std::sync::Arc;

    pub type Never = futures::never::Never;
    pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
    pub type ArcError = Arc<dyn std::error::Error + Send + Sync + 'static>;
}

pub use exports::*;
pub use imports::*;
