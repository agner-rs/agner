use crate::actor_id::ActorID;
use crate::exit_reason::ExitReason;

#[derive(Debug)]
pub enum CallMsg {
	Exit(ExitReason),
	Link(ActorID),
	Unlink(ActorID),
}
