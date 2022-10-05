use std::time::Duration;

const DEFAULT_INIT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy)]
pub enum InitType {
    NoAck,
    WithAck(WithAck),
}

#[derive(Debug, Clone, Copy)]
pub struct WithAck {
    pub init_timeout: Duration,
    pub stop_timeout: Duration,
}

impl InitType {
    pub fn no_ack() -> Self {
        Self::NoAck
    }
    pub fn with_ack() -> Self {
        Self::WithAck(Default::default())
    }
}

impl WithAck {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_init_timeout(self, init_timeout: Duration) -> Self {
        Self { init_timeout, ..self }
    }
    pub fn with_stop_timeout(self, stop_timeout: Duration) -> Self {
        Self { stop_timeout, ..self }
    }
}

impl Default for WithAck {
    fn default() -> Self {
        Self { init_timeout: DEFAULT_INIT_TIMEOUT, stop_timeout: DEFAULT_STOP_TIMEOUT }
    }
}

impl From<WithAck> for InitType {
    fn from(with_ack: WithAck) -> Self {
        Self::WithAck(with_ack)
    }
}
