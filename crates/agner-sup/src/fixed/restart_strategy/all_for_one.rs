use std::fmt;

use agner_actors::ActorID;

use crate::fixed::restart_strategy::{Decider, Frequency, RestartStrategy};

#[derive(Debug, Clone, Default)]
pub struct AllForOne {
    pub frequency: Frequency,
}

#[derive(Debug)]
pub struct AllForOneDecider {
    frequency: Frequency,
    children: Box<[ActorID]>,
}

impl RestartStrategy for AllForOne {
    type Decider = AllForOneDecider;

    fn new_decider(&self, children: Box<[ActorID]>) -> Self::Decider {
        AllForOneDecider { frequency: self.frequency, children }
    }
}

impl Decider for AllForOneDecider {}

impl fmt::Display for AllForOne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "all-for-one: {}", self.frequency)
    }
}
