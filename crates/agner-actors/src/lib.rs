mod actor;
mod actor_id;
mod actor_runner;
mod context;
mod exit_reason;
mod spawn_opts;
mod system;
mod system_config;

pub mod spsc;

mod exports {
    pub use crate::actor::{Actor, IntoExitReason};
    pub use crate::actor_id::ActorID;
    pub use crate::context::{Context, Event, Signal};
    pub use crate::exit_reason::ExitReason;
    pub use crate::spawn_opts::SpawnOpts;
    pub use crate::system::System;
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
