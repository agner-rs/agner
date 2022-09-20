use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{ActorID, Event, Exit};
use tokio::sync::oneshot;

pub type Tx<T> = oneshot::Sender<T>;

#[derive(Debug)]
pub enum Query<M> {
    Exit(ExitRq),

    SetLink(SetLinkRq),
    SetTrapExit(SetTrapExitRq),
    NextEvent(NextEventRq<M>),
}

#[derive(Debug)]
pub struct SetLinkRq {
    pub actor: ActorID,
    pub link: bool,
    pub reply_on_drop: Tx<Infallible>,
}

#[derive(Debug)]
pub struct ExitRq {
    pub reason: Exit,
    pub reply_on_drop: Tx<Infallible>,
}

#[derive(Debug)]
pub struct SetTrapExitRq {
    pub set_to: bool,
    pub reply_on_drop: Tx<Infallible>,
}

#[derive(Debug)]
pub struct NextEventRq<M> {
    pub timeout: Duration,
    pub reply_to: Tx<Event<M>>,
}

impl<M> From<SetLinkRq> for Query<M> {
    fn from(inner: SetLinkRq) -> Self {
        Self::SetLink(inner)
    }
}
impl<M> From<ExitRq> for Query<M> {
    fn from(inner: ExitRq) -> Self {
        Self::Exit(inner)
    }
}
impl<M> From<NextEventRq<M>> for Query<M> {
    fn from(inner: NextEventRq<M>) -> Self {
        Self::NextEvent(inner)
    }
}
impl<M> From<SetTrapExitRq> for Query<M> {
    fn from(inner: SetTrapExitRq) -> Self {
        Self::SetTrapExit(inner)
    }
}
