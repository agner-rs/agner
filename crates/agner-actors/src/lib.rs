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
    pub use crate::exit::{Exit, Shutdown};
    pub use crate::exit_handler::ExitHandler;
    pub use crate::spawn_opts::SpawnOpts;
    pub use crate::system::System;
    pub use crate::system_config::SystemConfig;

    pub use crate::actor_runner::ActorInfo;

    pub mod system_error {
        pub use crate::system::{SysChannelError, SysSpawnError};
    }

    pub mod exit_reason {
        pub use crate::exit::{BackendFailure, WellKnown};
    }

    /// Standard [exit-handlers](crate::exit_handler::ExitHandler)
    pub mod exit_handlers {
        pub use crate::exit_handler::{LogExitHandler, NoopExitHandler};
    }
}
mod imports {
    use std::sync::Arc;

    /// A type that cannot be instantiated.
    pub type Never = futures::never::Never;

    /// Boxed [standard error](std::error::Error)
    pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

    /// Arc-ed [standard error](std::error::Error)
    pub type ArcError = Arc<dyn std::error::Error + Send + Sync + 'static>;
}

pub use exports::*;
pub use imports::*;
