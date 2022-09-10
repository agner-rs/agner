use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorID {
	system: usize,
	actor: usize,
	serial: usize,
}

impl From<(usize, usize, usize)> for ActorID {
	fn from((system, actor, serial): (usize, usize, usize)) -> Self {
		Self { system, actor, serial }
	}
}
impl Into<(usize, usize, usize)> for ActorID {
	fn into(self) -> (usize, usize, usize) {
		(self.system, self.actor, self.serial)
	}
}

impl fmt::Display for ActorID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "<{}:{}:{}>", self.system, self.actor, self.serial)
	}
}
impl fmt::Debug for ActorID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ActorID({}:{}:{})", self.system, self.actor, self.serial)
	}
}
