use std::fmt;

use agner_actors::ActorID;

use crate::fixed::restart_strategy::{Decider, Frequency, RestartStrategy};

#[derive(Debug, Clone, Default)]
pub struct OneForOne {
    pub frequency: Frequency,
}

pub struct OneForOneDecider {
    frequency: Frequency,
    children: Box<[ActorID]>,
}

impl RestartStrategy for OneForOne {
    type Decider = OneForOneDecider;

    fn new_decider(&self, children: Box<[ActorID]>) -> Self::Decider {
        OneForOneDecider { frequency: self.frequency, children }
    }
}

impl Decider for OneForOneDecider {}

impl fmt::Display for OneForOne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "one-for-one: {}", self.frequency)
    }
}
