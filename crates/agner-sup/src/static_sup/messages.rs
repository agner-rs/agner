use agner_actor::{oneshot, ActorID};

#[derive(Debug)]
pub enum SupRq {
	GetChildID(usize, oneshot::Tx<Option<ActorID>>),
}
