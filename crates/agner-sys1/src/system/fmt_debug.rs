use std::fmt;

use super::*;

impl fmt::Debug for SystemOne {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<SystemOneImpl as fmt::Debug>::fmt(self.0.as_ref(), f)
	}
}

impl fmt::Debug for SystemOneImpl {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SystemOne")
			.field("system-id", &self.system_id)
			.field("max-actors", &self.actors.len())
			.finish()
	}
}
