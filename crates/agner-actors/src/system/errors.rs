/// A failure to spawn an actor by [`System::spawn(&self, ...)`](crate::system::System::spawn).
#[derive(Debug, thiserror::Error)]
pub enum SysSpawnError {
    #[error("No available IDs (max_actors limit reached)")]
    MaxActorsLimit,
}

/// An failure to open a channel to an actor (see [`System::channel::<Message>(&self,
/// ActorID)`](crate::system::System::channel))
#[derive(Debug, thiserror::Error)]
pub enum SysChannelError {
    #[error("No such actor")]
    NoActor,

    #[error("Invalid message-type")]
    InvalidMessageType,
}
