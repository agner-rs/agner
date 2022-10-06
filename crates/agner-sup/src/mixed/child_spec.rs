use std::time::Duration;

use agner_actors::Exit;

use crate::common::child_factory::ChildFactory;

pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_KILL_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy)]
pub enum ChildType {
    Permanent,
    Transient,
    Temporary,
}

#[derive(Debug)]
pub struct ChildSpec<ID> {
    pub id: ID,
    pub produce: Box<dyn ChildFactory<()>>,
    pub child_type: ChildType,
    pub shutdown: Vec<(Exit, Duration)>,
}

impl<ID> ChildSpec<ID> {
    pub fn new<P>(id: ID, produce: P) -> Self
    where
        P: ChildFactory<()>,
    {
        let produce = Box::new(produce);
        Self {
            id,
            produce,
            child_type: ChildType::Permanent,
            shutdown: vec![
                (Exit::shutdown(), DEFAULT_SHUTDOWN_TIMEOUT),
                (Exit::kill(), DEFAULT_KILL_TIMEOUT),
            ],
        }
    }
    pub fn with_child_type(self, restart: ChildType) -> Self {
        Self { child_type: restart, ..self }
    }
    pub fn with_shutdown(self, shutdown: impl IntoIterator<Item = (Exit, Duration)>) -> Self {
        let shutdown = shutdown.into_iter().collect();
        Self { shutdown, ..self }
    }
}
