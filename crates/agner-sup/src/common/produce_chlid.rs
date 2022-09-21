use crate::common::{InitType, StartChild};
use std::fmt;
use std::marker::PhantomData;

pub trait ProduceChild<Args>: fmt::Debug + Send + Sync + 'static {
    fn produce(&mut self, args: Args) -> Box<dyn StartChild>;
}

struct ProduceChildImpl<B, AF, M, PS> {
    actor_behaviour: B,
    actor_args_factory: AF,
    actor_message: PhantomData<M>,
    provided_services: PS,
    init_type: InitType,
}
