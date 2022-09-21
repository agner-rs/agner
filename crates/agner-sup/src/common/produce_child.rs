use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use agner_actors::{Actor, ActorID};

use crate::common;
use crate::common::{ArgsFactory, InitType, StartChild};
use crate::service::Service;

pub fn new<B, AF, M>(
    actor_behaviour: B,
    actor_args_factory: AF,

    init_type: InitType,
) -> Box<dyn ProduceChild<AF::Input>>
where
    AF: ArgsFactory,
    B: for<'a> Actor<'a, AF::Output, M>,
    M: Send + Sync + Unpin + 'static,
    B: Clone,
{
    let produce_child = ProduceChildImpl {
        actor_behaviour,
        actor_args_factory,
        actor_message: PhantomData::<M>,
        provided_services: Arc::new([]),
        init_type,
    };
    Box::new(produce_child)
}

pub trait ProduceChild<Args>: fmt::Debug + Send + Sync + 'static {
    fn produce(&mut self, sup_id: ActorID, args: Args) -> Box<dyn StartChild>;
}

struct ProduceChildImpl<B, AF, M> {
    actor_behaviour: B,
    actor_args_factory: AF,
    actor_message: PhantomData<M>,
    provided_services: Arc<[Service]>,
    init_type: InitType,
}

impl<B, AF, M> ProduceChild<AF::Input> for ProduceChildImpl<B, AF, M>
where
    AF: ArgsFactory,
    B: for<'a> Actor<'a, AF::Output, M>,

    B: Clone,
    M: Send + Sync + Unpin + 'static,
{
    fn produce(&mut self, sup_id: ActorID, args: AF::Input) -> Box<dyn StartChild> {
        let args = self.actor_args_factory.make_args(args);
        common::new_start_child(
            sup_id,
            self.actor_behaviour.to_owned(),
            args,
            self.init_type,
            self.provided_services.to_owned(),
        )
    }
}

impl<B, AF, M> fmt::Debug for ProduceChildImpl<B, AF, M>
where
    AF: ArgsFactory,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProduceChild")
            .field("input_arg_type", &std::any::type_name::<AF::Input>())
            .field("behavoiur", &std::any::type_name::<B>())
            .field("args", &std::any::type_name::<AF::Output>())
            .field("message", &std::any::type_name::<M>())
            .field("init_type", &self.init_type)
            .finish()
    }
}