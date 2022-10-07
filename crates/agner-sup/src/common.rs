//! The utility functions common to both [Uniform](crate::uniform) and [Mixed](crate::mixed)
//! supervisors.

use std::future::Future;
use std::pin::Pin;

use agner_actors::ActorID;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type StaticBoxedFuture<T> = BoxedFuture<'static, T>;

#[derive(Debug, Clone, Copy)]
pub struct ParentActor(pub ActorID);

mod start_child;
pub use start_child::{start_child, StartChildError};

mod stop_child;
pub use stop_child::{stop_child, ShutdownSequence, StopChildError};

mod init_type;
pub use init_type::{InitType, WithAck};

pub mod gen_child_spec;
pub use gen_child_spec::{CreateArgs, CreateChild, GenChildSpec};
