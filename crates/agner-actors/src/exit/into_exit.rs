use futures::never::Never;

use crate::exit::Exit;

impl From<()> for Exit {
    fn from((): ()) -> Self {
        Self::normal()
    }
}

impl From<Never> for Exit {
    fn from(infallible: Never) -> Self {
        unreachable!(
            "Whoa! We have an actual value of type `Infallible/Never`. Can I print it? Look — {:?}",
            infallible
        )
    }
}

impl<IntoExit> From<Option<IntoExit>> for Exit
where
    IntoExit: Into<Exit>,
{
    fn from(option: Option<IntoExit>) -> Self {
        option.map(Into::into).unwrap_or_else(|| Self::normal())
    }
}

impl<IntoExit1, IntoExit2> From<Result<IntoExit1, IntoExit2>> for Exit
where
    IntoExit1: Into<Exit>,
    IntoExit2: Into<Exit>,
{
    fn from(result: Result<IntoExit1, IntoExit2>) -> Self {
        match result {
            Ok(value) => value.into(),
            Err(value) => value.into(),
        }
    }
}
