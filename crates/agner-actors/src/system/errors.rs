#[derive(Debug, thiserror::Error)]
pub enum SysSpawnError {
    #[error("No available IDs (max_actors limit reached)")]
    MaxActorsLimit,
}

#[derive(Debug, thiserror::Error)]
pub enum SysChannelError {
    #[error("No such actor")]
    NoActor,

    #[error("Invalid message-type")]
    InvalidMessageType,
}
