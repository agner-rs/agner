use std::fmt;

use super::ActorID;

impl fmt::Display for ActorID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "<{}.{}.{}>", self.system(), self.actor(), self.seq())
	}
}
