use futures::Future;

use crate::actor_id::ActorID;
use crate::actor_runner::call_msg::CallMsg;
use crate::actor_runner::pipe::{PipeRx, PipeTx};
use crate::exit::Exit;
use crate::imports::Never;
use crate::init_ack::InitAckTx;
use crate::system::{System, SystemWeakRef};

/// Actor's API to itself
#[derive(Debug)]
pub struct Context<M> {
    actor_id: ActorID,
    system: SystemWeakRef,
    messages: PipeRx<M>,
    signals: PipeRx<Signal>,
    calls: PipeTx<CallMsg<M>>,
    init_ack_tx: Option<InitAckTx>,
}

#[derive(Debug)]
pub enum Event<M> {
    Message(M),
    Signal(Signal),
}

#[derive(Debug)]
pub enum Signal {
    Exit(ActorID, Exit),
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

    /// Receive next event (message or signal)
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

    /// Receive next message.
    pub async fn next_message(&mut self) -> M
    where
        M: Unpin,
    {
        self.messages.recv().await
    }

    /// Receive next signal.
    pub async fn next_signal(&mut self) -> Signal {
        self.signals.recv().await
    }
}

impl<M> Context<M> {
    #[deprecated(since = "0.3.2")]
    pub fn init_ack(&mut self, actor_id: Option<ActorID>) -> bool {
        self.init_ack_ok(actor_id)
    }
    pub fn init_ack_ok(&mut self, actor_id: Option<ActorID>) -> bool {
        if let Some(tx) = self.init_ack_tx.take() {
            let actor_id = actor_id.unwrap_or_else(|| self.actor_id());
            tx.ok(actor_id);
            true
        } else {
            false
        }
    }
    pub fn init_ack_err(&mut self, exit_reason: Exit) -> bool {
        if let Some(tx) = self.init_ack_tx.take() {
            tx.err(exit_reason);
            true
        } else {
            false
        }
    }

    pub async fn exit(&mut self, exit_reason: Exit) -> Never {
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
    pub async fn future_to_inbox<F>(&mut self, fut: F)
    where
        F: Future + Send + Sync + 'static,
        F::Output: Into<M>,
    {
        self.backend_call(CallMsg::FutureToInbox(Box::pin(async move {
            let out = fut.await;
            out.into()
        })))
        .await;
    }

    // pub async fn stream_to_inbox<S>(&mut self, stream: S)
    // where
    //     S: Stream,
    //     <S as Stream>::Item: Into<M>,
    //     S: Into<M>,
    // {
    //     unimplemented!()
    // }
}

impl<M> Context<M> {
    /// Create a new instance of [`Context`]
    pub(crate) fn new(
        actor_id: ActorID,
        system: SystemWeakRef,
        inbox: PipeRx<M>,
        signals: PipeRx<Signal>,
        calls: PipeTx<CallMsg<M>>,
        init_ack_tx: Option<InitAckTx>,
    ) -> Self {
        let calls = calls.blocking();
        Self { actor_id, system, messages: inbox, signals, calls, init_ack_tx }
    }
}

impl<M> Context<M> {
    async fn backend_call(&mut self, call: CallMsg<M>) {
        self.calls.send(call).await.expect("It's a blocking Tx. Should not reject.")
    }
}
