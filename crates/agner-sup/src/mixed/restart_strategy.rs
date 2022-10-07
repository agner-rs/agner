use agner_actors::{ActorID, Exit};
use std::error::Error as StdError;
use std::fmt;

mod common_decider;
mod strategies;
pub use strategies::{AllForOne, OneForOne, RestForOne};

#[cfg(test)]
mod tests;

use crate::mixed::child_spec::ChildType;

pub trait RestartStrategy<ID>: Clone + fmt::Debug + Send + 'static {
    type Decider;

    fn new_decider(&self, sup_id: ActorID) -> Self::Decider;
}

pub trait Decider<ID, D, I>: fmt::Debug + Send + 'static {
    type Error: StdError + Send + Sync + 'static;

    fn add_child(&mut self, id: ID, child_type: ChildType) -> Result<(), Self::Error>;
    fn rm_child(&mut self, id: ID) -> Result<(), Self::Error>;

    fn next_action(&mut self) -> Result<Option<Action<ID>>, Self::Error>;

    fn exit_signal(&mut self, actor_id: ActorID, exit: Exit, at: I) -> Result<(), Self::Error>;
    fn child_started(&mut self, id: ID, actor_id: ActorID) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum Action<ID> {
    Start(ID),
    Stop(ID),
    Shutdown(Exit),
}
