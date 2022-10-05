mod actor;
mod actor_id;
mod actor_runner;
mod context;
mod exit;
mod exit_handler;
mod spawn_opts;
mod system;
mod system_config;

mod exports {
    pub use crate::actor::Actor;
    pub use crate::actor_id::ActorID;
    pub use crate::context::{Context, Event, Signal};
    pub use crate::exit::Exit;
    pub use crate::exit_handler::ExitHandler;
    pub use crate::spawn_opts::SpawnOpts;
    pub use crate::system::{SysChannelError, SysSpawnError, System};
    pub use crate::system_config::SystemConfig;

    pub use crate::actor_runner::ActorInfo;

    pub mod exit_reason {
        pub use crate::exit::{BackendFailure, WellKnown};
    }

    pub mod exit_handlers {
        pub use crate::exit_handler::{LogExitHandler, NoopExitHandler};
    }
}
mod imports {
    use std::sync::Arc;

    pub type Never = futures::never::Never;
    pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
    pub type ArcError = Arc<dyn std::error::Error + Send + Sync + 'static>;
}

pub use exports::*;
pub use imports::*;
