use std::fmt;
use std::marker::PhantomData;

use agner_actors::{Actor, ActorID, System};
use agner_registered::Service;
use futures::TryFutureExt;

use crate::common::{ArgsFactory, InitType, StaticBoxedFuture, WithRegisteredService};

use crate::common::StartChildError;

mod registered;
mod start_child;

#[cfg(test)]
mod tests;

pub fn new<B, AF, IT, M>(
    actor_behaviour: B,
    actor_args_factory: AF,

    init_type: IT,
) -> impl ProduceChild<AF::Input> + WithRegisteredService
where
    AF: ArgsFactory,
    IT: Into<InitType>,
    B: for<'a> Actor<'a, AF::Output, M>,
    M: Send + Sync + Unpin + 'static,
    B: Clone,
{
    let init_type = init_type.into();
    let produce_child = ProduceChildImpl {
        actor_behaviour,
        actor_args_factory,
        actor_message: PhantomData::<M>,
        init_type,

        registered_service: None,
    };
    // Box::new(produce_child)
    produce_child
}

pub trait ProduceChild<Args>: fmt::Debug + Send + Sync + 'static {
    fn produce(
        &mut self,
        system: System,
        sup_id: ActorID,
        args: Args,
    ) -> StaticBoxedFuture<Result<ActorID, StartChildError>>;
}

struct ProduceChildImpl<B, AF, M> {
    actor_behaviour: B,
    actor_args_factory: AF,
    actor_message: PhantomData<M>,
    init_type: InitType,

    registered_service: Option<Service>,
}

// impl<Args> ProduceChild<Args> for Box<dyn ProduceChild<Args>>
// where
//     Args: Send + Sync + 'static,
// {
//     fn produce(&mut self, sup_id: ActorID, args: Args) -> Box<dyn StartChild> {
//         self.as_mut().produce(sup_id, args)
//     }
// }

impl<B, AF, M> ProduceChild<AF::Input> for ProduceChildImpl<B, AF, M>
where
    AF: ArgsFactory,
    B: for<'a> Actor<'a, AF::Output, M>,

    B: Clone,
    M: Send + Sync + Unpin + 'static,
{
    fn produce(
        &mut self,
        system: System,
        sup_id: ActorID,
        args: AF::Input,
    ) -> StaticBoxedFuture<Result<ActorID, StartChildError>> {
        let args = self.actor_args_factory.make_args(args);
        let behaviour = self.actor_behaviour.to_owned();
        let init_type = self.init_type;

        // FIXME: feature
        let registered_service = self.registered_service.to_owned();

        Box::pin(
            start_child::do_start_child(system.to_owned(), sup_id, behaviour, args, init_type)
                .and_then(move |child_id| async move {
                    // FIXME: feature
                    if let Some(service) = registered_service {
                        let reg_guard = service.register(child_id).await;
                        system.put_data(child_id, reg_guard).await;
                    }

                    Ok(child_id)
                }),
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
