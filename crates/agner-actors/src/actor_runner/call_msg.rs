use crate::{ActorID, ExitReason};

#[derive(Debug)]
pub enum CallMsg {
	Exit(ExitReason),
	Link(ActorID),
	Unlink(ActorID),
}
