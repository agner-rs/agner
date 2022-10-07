use crate::common::{GenChildSpec, ShutdownSequence};

pub type UniformChildSpec<B, A, M> = GenChildSpec<B, A, M, Ext>;

impl UniformChildSpec<(), (), ()> {
    pub fn uniform() -> Self {
        Self::from_ext(Default::default())
    }
}

impl<B, A, M> UniformChildSpec<B, A, M> {
    pub fn with_shutdown_sequence(mut self, shutdown_sequence: ShutdownSequence) -> Self {
        self.ext_mut().shutdown_sequence = shutdown_sequence;
        self
    }
    pub fn shutdown_sequence(&self) -> &ShutdownSequence {
        &self.ext().shutdown_sequence
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ext {
    shutdown_sequence: ShutdownSequence,
}
