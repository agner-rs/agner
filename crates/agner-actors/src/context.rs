use std::any::{Any, TypeId};
use std::collections::HashMap;

use futures::Future;

use crate::actor_id::ActorID;
use crate::actor_runner::call_msg::CallMsg;
use crate::actor_runner::pipe::{PipeRx, PipeTx};
use crate::exit::Exit;
use crate::imports::Never;
use crate::system::{System, SystemWeakRef};

/// Actor's API to itself
#[derive(Debug)]
pub struct Context<M> {
    actor_id: ActorID,
    system: SystemWeakRef,
    messages: PipeRx<M>,
    signals: PipeRx<Signal>,
    calls: PipeTx<CallMsg<M>>,
    data: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
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
    pub fn put<D>(&mut self, data: D) -> Option<D>
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = data.type_id();
        let boxed = Box::new(data);
        let prev_opt = self.data.insert(type_id, boxed);

        prev_opt
            .map(|any| any.downcast().expect("The value does not match the type-id."))
            .map(|b| *b)
    }
    pub fn take<D>(&mut self) -> Option<D>
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<D>();
        let boxed_opt = self.data.remove(&type_id);

        boxed_opt
            .map(|any| any.downcast().expect("The value does not match the type-id."))
            .map(|b| *b)
    }
    pub fn get<D>(&self) -> Option<&D>
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<D>();
        let boxed_opt = self.data.get(&type_id);

        boxed_opt.map(|any| any.downcast_ref().expect("The value does not match the type-id."))
    }
    pub fn get_mut<D>(&mut self) -> Option<&mut D>
    where
        D: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<D>();
        let boxed_opt = self.data.get_mut(&type_id);

        boxed_opt.map(|any| any.downcast_mut().expect("The value does not match the type-id."))
    }
    pub fn with_data(
        mut self,
        data: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
    ) -> Self {
        self.data = data;
        self
    }
}

impl<M> Context<M> {
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
}

impl<M> Context<M> {
    /// Create a new instance of [`Context`]
    pub(crate) fn new(
        actor_id: ActorID,
        system: SystemWeakRef,
        inbox: PipeRx<M>,
        signals: PipeRx<Signal>,
        calls: PipeTx<CallMsg<M>>,
    ) -> Self {
        let calls = calls.blocking();
        Self { actor_id, system, messages: inbox, signals, calls, data: Default::default() }
    }
}

impl<M> Context<M> {
    async fn backend_call(&mut self, call: CallMsg<M>) {
        self.calls.send(call).await.expect("It's a blocking Tx. Should not reject.")
    }
}
