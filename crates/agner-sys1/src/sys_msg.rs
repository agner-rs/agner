use agner_actor::error::{ExitError, ExitReason};
use agner_actor::{oneshot, ActorID};

#[derive(Debug)]
pub enum SysMsg {
	Shutdown(ExitReason),

	Exit(ActorID, ExitError),
	Link(ActorID),
	Unlink(ActorID),
	Wait(oneshot::Tx<ExitReason>),
}
