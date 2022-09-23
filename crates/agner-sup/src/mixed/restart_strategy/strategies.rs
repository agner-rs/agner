use std::time::{Duration, Instant};

use crate::mixed::child_id::ChildID;
use crate::mixed::restart_intensity::RestartIntensity;

use super::common_decider::{CommonDecider, RestartType};
use super::RestartStrategy;

#[derive(Debug, Clone)]
pub struct OneForOne {
    restart_intensity: RestartIntensity<Duration>,
}

#[derive(Debug, Clone)]
pub struct AllForOne {
    restart_intensity: RestartIntensity<Duration>,
}

#[derive(Debug, Clone)]
pub struct RestForOne {
    restart_intensity: RestartIntensity<Duration>,
}

impl OneForOne {
    pub fn new(restart_intensity: RestartIntensity<Duration>) -> Self {
        Self { restart_intensity }
    }
}

impl AllForOne {
    pub fn new(restart_intensity: RestartIntensity<Duration>) -> Self {
        Self { restart_intensity }
    }
}

impl RestForOne {
    pub fn new(restart_intensity: RestartIntensity<Duration>) -> Self {
        Self { restart_intensity }
    }
}

impl<ID> RestartStrategy<ID> for OneForOne
where
    ID: ChildID,
{
    type Decider = CommonDecider<ID, Duration, Instant>;

    fn new_decider(&self, sup_id: agner_actors::ActorID) -> Self::Decider {
        CommonDecider::new(sup_id, RestartType::One, self.restart_intensity.to_owned())
    }
}

impl<ID> RestartStrategy<ID> for AllForOne
where
    ID: ChildID,
{
    type Decider = CommonDecider<ID, Duration, Instant>;

    fn new_decider(&self, sup_id: agner_actors::ActorID) -> Self::Decider {
        CommonDecider::new(sup_id, RestartType::All, self.restart_intensity.to_owned())
    }
}

impl<ID> RestartStrategy<ID> for RestForOne
where
    ID: ChildID,
{
    type Decider = CommonDecider<ID, Duration, Instant>;

    fn new_decider(&self, sup_id: agner_actors::ActorID) -> Self::Decider {
        CommonDecider::new(sup_id, RestartType::Rest, self.restart_intensity.to_owned())
    }
}
