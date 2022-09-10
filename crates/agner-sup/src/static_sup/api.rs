use agner_actor::error::AnyError;
use agner_actor::{oneshot, ActorID, System};

use super::messages::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StaticSupApi(ActorID);

impl From<ActorID> for StaticSupApi {
	fn from(id: ActorID) -> Self {
		Self(id)
	}
}

impl StaticSupApi {
	pub async fn get_child_by_idx<Sys: System>(
		&self,
		system: &Sys,
		idx: usize,
	) -> Result<Option<ActorID>, AnyError> {
		let (tx, rx) = oneshot::channel();
		let request = SupRq::GetChildID(idx, tx);
		system.send(self.0, request);
		let response = rx.await.ok_or("HUP")?;
		Ok(response)
	}
}
