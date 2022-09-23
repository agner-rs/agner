mod child_id;
mod child_spec;
mod restart_intensity;
mod restart_strategy;
mod sup_spec;

pub use child_id::ChildID;
pub use child_spec::{ChildSpec, ChildType};
pub use restart_intensity::RestartIntensity;
pub use restart_strategy::{AllForOne, OneForOne, RestForOne};
pub use sup_spec::SupSpec;
pub mod supervisor;

pub mod plumbing {
    pub use super::restart_intensity::{DurationToInstant, ElapsedSince, RestartStats};
    pub use super::restart_strategy::{Action, Decider, RestartStrategy};
}
