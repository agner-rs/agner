use crate::common::{GenChildSpec, ShutdownSequence};

mod flat_mixed_child_spec;
pub use flat_mixed_child_spec::FlatMixedChildSpec;

pub type MixedChildSpec<ID, B, A, M> = GenChildSpec<B, A, M, Ext<ID>>;

pub type BoxedMixedChildSpec<ID> = Box<dyn FlatMixedChildSpec<ID>>;

#[derive(Debug, Clone)]
pub struct Ext<ID> {
    id: ID,
    child_type: ChildType,
    shutdown: ShutdownSequence,
}

#[derive(Debug, Clone, Copy)]
pub enum ChildType {
    Permanent,
    Transient,
    Temporary,
}

impl<ID> MixedChildSpec<ID, (), (), ()> {
    pub fn mixed(id: ID) -> Self {
        let ext = Ext { id, child_type: ChildType::Permanent, shutdown: Default::default() };

        Self::from_ext(ext)
    }
}
impl<ID, B, A, M> MixedChildSpec<ID, B, A, M> {
    pub fn child_type(mut self, child_type: ChildType) -> Self {
        self.ext_mut().child_type = child_type;
        self
    }
    pub fn shutdown(mut self, shutdown: ShutdownSequence) -> Self {
        self.ext_mut().shutdown = shutdown;
        self
    }
}
