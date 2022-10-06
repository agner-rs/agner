use std::time::Duration;

use agner_actors::Exit;

use crate::common::GenChildSpec;

mod flat_mixed_child_spec;
pub use flat_mixed_child_spec::FlatMixedChildSpec;

pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_KILL_TIMEOUT: Duration = Duration::from_secs(5);

pub type MixedChildSpec<ID, B, A, M> = GenChildSpec<B, A, M, Ext<ID>>;

#[derive(Debug, Clone)]
pub struct Ext<ID> {
    id: ID,
    child_type: ChildType,
    shutdown: Vec<(Exit, Duration)>,
}

#[derive(Debug, Clone, Copy)]
pub enum ChildType {
    Permanent,
    Transient,
    Temporary,
}

impl<ID> MixedChildSpec<ID, (), (), ()> {
    pub fn id(id: ID) -> Self {
        let ext = Ext {
            id,
            child_type: ChildType::Permanent,
            shutdown: vec![
                (Exit::shutdown(), DEFAULT_SHUTDOWN_TIMEOUT),
                (Exit::kill(), DEFAULT_KILL_TIMEOUT),
            ],
        };

        Self::from_ext(ext)
    }
}
impl<ID, B, A, M> MixedChildSpec<ID, B, A, M> {
    pub fn child_type(mut self, child_type: ChildType) -> Self {
        self.ext_mut().child_type = child_type;
        self
    }
    pub fn shutdown(mut self, shutdown: Vec<(Exit, Duration)>) -> Self {
        self.ext_mut().shutdown = shutdown;
        self
    }
}
