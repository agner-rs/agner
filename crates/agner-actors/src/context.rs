use std::sync::Arc;

use futures::Future;

use crate::actor_id::ActorID;
use crate::actor_runner::call_msg::CallMsg;
use crate::actor_runner::pipe::{PipeRx, PipeTx};
use crate::exit_reason::ExitReason;
use crate::imports::Never;
use crate::system::{System, SystemOpt};

/// Actor's API to itself
#[derive(Debug)]
pub struct Context<M> {
    actor_id: ActorID,
    system: SystemOpt,
    messages: PipeRx<M>,
    signals: PipeRx<Signal>,
    calls: PipeTx<CallMsg<M>>,
}

#[derive(Debug)]
pub enum Event<M> {
    Message(M),
    Signal(Signal),
}

#[derive(Debug)]
pub enum Signal {
    Exited(ActorID, Arc<ExitReason>),
}

impl<M> Context<M> {
    /// Get current actor's [`ActorID`]
    pub fn actor_id(&self) -> ActorID {
        self.actor_id
    }

    /// Get the [`System`] this actor is running in.
    pub fn system(&self) -> System {
        self.system.rc_upgrade().expect("System gone")
    }

    pub async fn next_event(&mut self) -> Event<M>
    where
        M: Unpin,
    {
        tokio::select! {
            biased;

            signal = self.signals.recv() =>
                Event::Signal(signal),
            message = self.messages.recv() =>
                Event::Message(message),
        }
    }
}

impl<M> Context<M> {
    pub async fn exit(&mut self, exit_reason: ExitReason) -> Never {
        self.backend_call(CallMsg::Exit(exit_reason)).await;
        std::future::pending().await
    }
    pub async fn link(&mut self, to: ActorID) {
        self.backend_call(CallMsg::Link(to)).await;
    }
    pub async fn unlink(&mut self, from: ActorID) {
        self.backend_call(CallMsg::Unlink(from)).await;
    }
    pub async fn trap_exit(&mut self, trap_exit: bool) {
        self.backend_call(CallMsg::TrapExit(trap_exit)).await;
    }
    pub async fn pipe_to_inbox<F>(&mut self, fut: F)
    where
        F: Future + Send + Sync + 'static,
        F::Output: Into<M>,
    {
        self.backend_call(CallMsg::PipeToInbox(Box::pin(async move {
            let out = fut.await;
            out.into()
        })))
        .await;
    }
}

impl<M> Context<M> {
    /// Create a new instance of [`Context`]
    pub(crate) fn new(
        actor_id: ActorID,
        system: SystemOpt,
        inbox: PipeRx<M>,
        signals: PipeRx<Signal>,
        calls: PipeTx<CallMsg<M>>,
    ) -> Self {
        Self { actor_id, system, messages: inbox, signals, calls }
    }
}

impl<M> Context<M> {
    async fn backend_call(&mut self, call: CallMsg<M>) {
        if let Err(reason) = self.calls.send(call).await {
            panic!("Failed to perform backend-call");
        }
    }
}
