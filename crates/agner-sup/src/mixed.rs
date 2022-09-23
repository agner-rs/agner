mod child_id;
mod child_spec;
mod restart_intensity;
mod restart_strategy;
mod sup_spec;

use agner_actors::{ActorID, Exit, System};
use agner_utils::result_err_flatten::ResultErrFlattenIn;
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

pub use supervisor::run;
use tokio::sync::oneshot;

use self::supervisor::SupervisorError;

pub async fn start_child<ID>(
    system: &System,
    sup: ActorID,
    child_spec: ChildSpec<ID>,
) -> Result<ActorID, SupervisorError>
where
    ID: ChildID,
{
    let (tx, rx) = oneshot::channel();
    let message = supervisor::Message::StartChild(child_spec, tx);
    system.send(sup, message).await;
    rx.await.err_flatten_in()
}

pub async fn terminate_child<ID>(
    system: &System,
    sup: ActorID,
    child_id: ID,
) -> Result<Exit, SupervisorError>
where
    ID: ChildID,
{
    let (tx, rx) = oneshot::channel();
    let message = supervisor::Message::TerminateChild(child_id, tx);
    system.send(sup, message).await;
    rx.await.err_flatten_in()
}

pub async fn which_children<ID>(
    system: &System,
    sup: ActorID,
) -> Result<Vec<(ID, ActorID)>, SupervisorError>
where
    ID: ChildID,
{
    let (tx, rx) = oneshot::channel();
    let message = supervisor::Message::WhichChildren(tx);
    system.send(sup, message).await;
    rx.await.map_err(Into::into)
}
