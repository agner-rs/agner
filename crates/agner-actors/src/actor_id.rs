mod impl_fmt;

#[cfg(test)]
mod tests;

/// Identifier of an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorID(Inner);

impl ActorID {
    /// Create a new [`ActorID`] from the provided components
    pub(crate) fn new(system: usize, actor: usize, seq: usize) -> Self {
        Self(Inner { system, actor, seq })
    }
    /// Get `system` component
    pub(crate) fn system(&self) -> usize {
        self.0.system
    }
    /// Get `actor` component
    pub(crate) fn actor(&self) -> usize {
        self.0.actor
    }
    /// Get `seq` component
    pub(crate) fn seq(&self) -> usize {
        self.0.seq
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Inner {
    system: usize,
    actor: usize,
    seq: usize,
}
