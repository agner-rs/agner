use std::fmt;
use std::time::Duration;

use agner_actors::Exit;

use crate::common::gen_child_spec::CreateChild;
use crate::mixed::child_spec::MixedChildSpec;
use crate::mixed::ChildID;

use super::ChildType;

pub trait FlatMixedChildSpec<ID>:
    CreateChild<Args = ()> + fmt::Debug + Unpin + Send + Sync + 'static
{
    fn id(&self) -> ID;
    fn child_type(&self) -> ChildType;
    fn shutdown(&self) -> &[(Exit, Duration)];
}

impl<ID, B, A, M> FlatMixedChildSpec<ID> for MixedChildSpec<ID, B, A, M>
where
    ID: ChildID,
    Self: CreateChild<Args = ()>,
    A: fmt::Debug,
    B: Unpin + Send + Sync + 'static,
    A: Unpin + Send + Sync + 'static,
    M: Unpin + Send + Sync + 'static,
{
    fn id(&self) -> ID {
        self.ext().id
    }
    fn child_type(&self) -> ChildType {
        self.ext().child_type
    }
    fn shutdown(&self) -> &[(Exit, Duration)] {
        self.ext().shutdown.as_ref()
    }
}

impl<ID, B, A, M> From<MixedChildSpec<ID, B, A, M>> for Box<dyn FlatMixedChildSpec<ID>>
where
    MixedChildSpec<ID, B, A, M>: FlatMixedChildSpec<ID>,
{
    fn from(cs: MixedChildSpec<ID, B, A, M>) -> Self {
        Box::new(cs)
    }
}
