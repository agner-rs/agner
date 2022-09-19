mod behaviour;
mod child_spec;
mod restart_strategy;
mod sup_spec;

type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

pub mod hlist;

use std::future::Future;
use std::pin::Pin;

pub use behaviour::{fixed_sup, Message};
pub use child_spec::{args_arc, args_call, args_clone, child_spec, ArgsFactory, ChildSpec};

#[allow(deprecated)]
pub use child_spec::{arg_arc, arg_call, arg_clone};

pub use restart_strategy::{AllForOne, Decider, OneForOne, RestartStrategy};
pub use sup_spec::SupSpec;
