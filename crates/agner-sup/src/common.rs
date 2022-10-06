use std::future::Future;
use std::pin::Pin;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'a>>;
pub type StaticBoxedFuture<T> = BoxedFuture<'static, T>;

#[derive(Debug, Clone, Copy)]
pub struct ParentActor(pub ActorID);

pub mod util;

use agner_actors::ActorID;

mod init_type;
pub use init_type::{InitType, WithAck};

mod start_child_error;
pub use start_child_error::StartChildError;

pub mod args_factory;
pub use args_factory::ArgsFactory;

mod stop_child;

pub mod child_factory;
pub use child_factory::ChildFactory;

#[deprecated(since = "0.3.10")]
pub use child_factory as produce_child;
#[deprecated(since = "0.3.10")]
pub use child_factory::ChildFactory as ProduceChild;

#[cfg(feature = "reg")]
mod with_registered_service;

#[cfg(feature = "reg")]
pub use with_registered_service::WithRegisteredService;
