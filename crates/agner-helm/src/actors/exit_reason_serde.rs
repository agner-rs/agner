pub enum WellKnown {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[error("{}", message)]
pub struct GenericError {
    pub message: String,

    #[source]
    pub source: Option<Box<Self>>,
}
