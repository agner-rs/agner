use std::fmt;

use crate::actor_runner::ActorRunner;

impl<Message> fmt::Debug for ActorRunner<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActorRunner")
            .field("message_type", &std::any::type_name::<Message>())
            .field("actor_id", &self.actor_id)
            .finish()
    }
}
