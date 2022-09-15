use std::time::Duration;


pub trait RestartStrategy {
    type Decider: Decider;
    
}

pub trait Decider {}

#[derive(Debug, Clone, Default)]
pub struct Frequency {
    pub max_restarts: usize,
    pub within: Duration,
}

#[derive(Debug, Clone)]
pub struct OneForOne {
    pub frequency: Frequency,
}

#[derive(Debug, Clone)]
pub struct AllForOne {
    pub frequency: Frequency,
}

