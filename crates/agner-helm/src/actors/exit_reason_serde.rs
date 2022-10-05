use std::error::Error as StdError;

use agner_actors::exit_reason::WellKnown;
use agner_actors::{ActorID, BoxError, Exit};

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

    #[serde(rename = "linked")]
    Linked(ActorID, Box<ExitSerde>),

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

impl From<ExitSerde> for Exit {
    fn from(from: ExitSerde) -> Self {
        match from {
            ExitSerde::Backend(ge) => Self::Custom(BoxError::from(ge).into()),
            ExitSerde::Custom(ge) => Self::Custom(BoxError::from(ge).into()),
            ExitSerde::Standard(se) => Self::Standard(se.into()),
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

impl From<ExitStandardSerde> for WellKnown {
    fn from(from: ExitStandardSerde) -> Self {
        match from {
            ExitStandardSerde::Normal => Self::Normal,
            ExitStandardSerde::Kill => Self::Kill,
            ExitStandardSerde::Linked(actor_id, reason) =>
                Self::Linked(actor_id, Box::new((*reason).into())),
            ExitStandardSerde::NoActor => Self::NoActor,
            ExitStandardSerde::Shutdown(source) =>
                Self::Shutdown(source.map(BoxError::from).map(Into::into)),
        }
    }
}

impl From<WellKnown> for ExitStandardSerde {
    fn from(exit_standard: WellKnown) -> Self {
        match exit_standard {
            WellKnown::Normal => Self::Normal,
            WellKnown::Kill => Self::Kill,
            WellKnown::Linked(actor_id, reason) =>
                Self::Linked(actor_id, Box::new((*reason).into())),
            WellKnown::NoActor => Self::NoActor,
            WellKnown::Shutdown(reason) => Self::Shutdown(
                reason.as_ref().map(AsRef::as_ref).map(GenericError::from_std_error_send_sync),
            ),
        }
    }
}
