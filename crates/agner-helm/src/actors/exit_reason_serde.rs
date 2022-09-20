use std::error::Error as StdError;

use agner_actors::{ActorID, BoxError, Exit, ExitStandard};

// pub struct ExitSerde<'a>(&'a Exit);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum ExitSerde {
    #[serde(rename = "standard")]
    Standard(ExitStandardSerde),

    #[serde(rename = "backend")]
    Backend(GenericError),

    #[serde(rename = "custom")]
    Custom(GenericError),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum ExitStandardSerde {
    #[serde(rename = "normal")]
    Normal,

    #[serde(rename = "kill")]
    Kill,

    #[serde(rename = "exited")]
    Exited(ActorID, Box<ExitSerde>),

    #[serde(rename = "no_actor")]
    NoActor,

    #[serde(rename = "shutdown")]
    Shutdown(Option<GenericError>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[error("{}", message)]
pub struct GenericError {
    pub message: String,

    #[source]
    pub source: Option<Box<Self>>,
}

impl GenericError {
    fn from_std_error(other: &(dyn StdError + 'static)) -> Self {
        Self {
            message: format!("{}", other),
            source: other.source().map(GenericError::from_std_error).map(Box::new),
        }
    }

    fn from_std_error_send_sync(other: &(dyn StdError + Send + Sync + 'static)) -> Self {
        Self {
            message: format!("{}", other),
            source: other.source().map(GenericError::from_std_error).map(Box::new),
        }
    }
}

impl Into<Exit> for ExitSerde {
    fn into(self) -> Exit {
        match self {
            Self::Backend(ge) => Exit::Custom(BoxError::from(ge).into()),
            Self::Custom(ge) => Exit::Custom(BoxError::from(ge).into()),
            Self::Standard(se) => Exit::Standard(se.into()),
        }
    }
}
impl From<Exit> for ExitSerde {
    fn from(exit: Exit) -> Self {
        match exit {
            Exit::Standard(se) => Self::Standard(se.into()),
            Exit::Backend(be) => Self::Backend(GenericError::from_std_error(&be)),
            Exit::Custom(be) => Self::Custom(GenericError::from_std_error(&be)),
        }
    }
}

impl From<ExitStandardSerde> for ExitStandard {
    fn from(from: ExitStandardSerde) -> Self {
        match from {
            ExitStandardSerde::Normal => Self::Normal,
            ExitStandardSerde::Kill => Self::Kill,
            ExitStandardSerde::Exited(actor_id, reason) =>
                Self::Exited(actor_id, Box::new((*reason).into())),
            ExitStandardSerde::NoActor => Self::NoActor,
            ExitStandardSerde::Shutdown(source) =>
                Self::Shutdown(source.map(BoxError::from).map(Into::into)),
        }
    }
}

impl From<ExitStandard> for ExitStandardSerde {
    fn from(exit_standard: ExitStandard) -> Self {
        match exit_standard {
            ExitStandard::Normal => Self::Normal,
            ExitStandard::Kill => Self::Kill,
            ExitStandard::Exited(actor_id, reason) =>
                Self::Exited(actor_id, Box::new((*reason).into())),
            ExitStandard::NoActor => Self::NoActor,
            ExitStandard::Shutdown(reason) => Self::Shutdown(
                reason.as_ref().map(AsRef::as_ref).map(GenericError::from_std_error_send_sync),
            ),
        }
    }
}
