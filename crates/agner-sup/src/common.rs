use std::future::Future;
use std::pin::Pin;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type StaticBoxedFuture<T> = BoxedFuture<'static, T>;

#[derive(Debug, Clone, Copy)]
pub struct ParentActor(pub ActorID);

pub mod util;

use agner_actors::ActorID;

mod init_type;
pub use init_type::{InitType, WithAck};

mod start_child_error;
pub use start_child_error::StartChildError;

mod stop_child;

pub mod gen_child_spec;
pub use gen_child_spec::{CreateArgs, CreateChild, GenChildSpec};
