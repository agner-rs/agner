mod actor;
mod actor_id;
mod actor_runner;
mod context;
mod exit_reason;
mod system;
mod system_config;

pub mod spsc;

use std::sync::Arc;

pub use actor::Actor;
pub use actor_id::ActorID;
pub use context::{Context, Event};
pub use exit_reason::ExitReason;
pub use futures::never::Never;
pub use system::System;
pub use system_config::SystemConfig;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type ArcError = Arc<dyn std::error::Error + Send + Sync + 'static>;
