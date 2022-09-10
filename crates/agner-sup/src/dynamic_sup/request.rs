use agner_actor::{oneshot, ActorID};

use super::{StartError, StopError};

#[derive(Debug)]
pub enum SupRq<Rq> {
	StartChild(Rq, oneshot::Tx<Result<ActorID, StartError>>),
	StopChild(ActorID, oneshot::Tx<Result<(), StopError>>),
}
