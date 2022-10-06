use agner_actors::{ActorID, System};

use crate::common::{StartChildError, StaticBoxedFuture};

pub trait CreateChild {
    type Args;

    fn create_child(
        &mut self,
        system: &System,
        sup_id: ActorID,
        args: Self::Args,
    ) -> StaticBoxedFuture<Result<ActorID, StartChildError>>;
}

pub trait CreateArgs {
    type Input;
    type Output;

    fn create_args(&mut self, input: Self::Input) -> Self::Output;
}
