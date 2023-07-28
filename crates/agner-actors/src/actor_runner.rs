use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use agner_utils::std_error_pp::StdErrorPP;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};
use tracing::Instrument;

use crate::actor::Actor;
use crate::actor_id::ActorID;
use crate::context::{Context, Signal};
use crate::exit::{BackendFailure, Exit};
use crate::exit_handler::ExitHandler;
use crate::spawn_opts::SpawnOpts;
use crate::system::SystemWeakRef;

pub(crate) mod call_msg;
mod impl_debug;
pub(crate) mod pipe;
pub(crate) mod sys_msg;
mod watches;

use call_msg::CallMsg;
use sys_msg::SysMsg;
use watches::Watches;

use self::pipe::{PipeRx, PipeTx};
pub use self::sys_msg::ActorInfo;

pub(crate) struct ActorRunner<Message> {
    pub actor_id: ActorID,
    pub system_opt: SystemWeakRef,
    pub messages_rx: mpsc::UnboundedReceiver<Message>,
    pub sys_msg_rx: mpsc::UnboundedReceiver<SysMsg>,
    pub sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    pub exit_handler: Arc<dyn ExitHandler>,
    pub spawn_opts: SpawnOpts,
}

impl<Message> ActorRunner<Message>
where
    Message: Unpin,
{
    #[tracing::instrument(skip_all, fields(
        actor_id = display(self.actor_id),
        behaviour = std::any::type_name::<Behaviour>(),
        msg_type = std::any::type_name::<Message>(),
    ))]
    pub async fn run<Behaviour, Args>(self, behaviour: Behaviour, args: Args)
    where
        for<'a> Behaviour: Actor<'a, Args, Message>,
    {
        let Self {
            actor_id,
            system_opt,
            messages_rx,
            sys_msg_rx,
            sys_msg_tx,
            exit_handler,
            mut spawn_opts,
        } = self;

        tracing::trace!(
            "init [m-inbox: {:?}, s-inbox: {:?}, msg-type: {}]",
            spawn_opts.msg_inbox_size(),
            spawn_opts.sig_inbox_size(),
            std::any::type_name::<Message>()
        );

        let (inbox_w, inbox_r) = pipe::new::<Message>(spawn_opts.msg_inbox_size());
        let (signals_w, signals_r) = pipe::new::<Signal>(spawn_opts.sig_inbox_size());
        let (calls_w, calls_r) = pipe::new::<CallMsg<Message>>(1);
        let mut context =
            Context::new(actor_id, system_opt.to_owned(), inbox_r, signals_r, calls_w)
                .with_data(spawn_opts.take_data());

        let behaviour_running = async move {
            let exit_reason = behaviour
                .run(&mut context, args)
                .instrument(tracing::span!(tracing::Level::TRACE, "<behaviour as Actor>::run"))
                .await
                .into();
            context
                .exit(exit_reason.clone())
                .instrument(tracing::span!(tracing::Level::TRACE, "Context::exit"))
                .await;
            unreachable!()
        };

        let mut actor_backend = Backend {
            actor_id,
            system_opt: system_opt.to_owned(),
            sys_msg_rx,
            sys_msg_tx,
            messages_rx,
            inbox_w,
            signals_w,
            calls_r,
            watches: Default::default(),
            tasks: FuturesUnordered::<
                Pin<Box<dyn Future<Output = Option<Message>> + Send + Sync + 'static>>,
            >::new(),

            exit_handler,

            actor_type_info: (
                std::any::type_name::<Behaviour>(),
                std::any::type_name::<Args>(),
                std::any::type_name::<Message>(),
            ),
        };

        for link_to in spawn_opts.links() {
            actor_backend.do_link(link_to).await;
        }

        let actor_backend_running = actor_backend.run_actor_backend();

        tracing::trace!("running...");
        let exit_reason = tokio::select! {
            biased;

            exit_reason = actor_backend_running => exit_reason,
            _ = behaviour_running => unreachable!("Future<Output = Infallible> has returned"),
        };
        tracing::trace!("exited: {}", exit_reason.pp());

        if let Some(system) = system_opt.rc_upgrade() {
            tracing::trace!("cleaning up actor-entry...");
            system.actor_entry_terminate(actor_id, exit_reason).await;
        }
    }
}

struct Backend<Message> {
    actor_id: ActorID,
    system_opt: SystemWeakRef,
    sys_msg_rx: mpsc::UnboundedReceiver<SysMsg>,
    sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    messages_rx: mpsc::UnboundedReceiver<Message>,
    inbox_w: PipeTx<Message>,
    signals_w: PipeTx<Signal>,
    calls_r: PipeRx<CallMsg<Message>>,
    watches: Watches,
    tasks: FuturesUnordered<Pin<Box<dyn Future<Output = Option<Message>> + Send + Sync + 'static>>>,
    exit_handler: Arc<dyn ExitHandler>,

    actor_type_info: (&'static str, &'static str, &'static str),
}

impl<Message> Backend<Message>
where
    Message: Unpin,
{
    #[tracing::instrument(skip_all)]
    async fn run_actor_backend(mut self) -> Exit {
        tracing::trace!("running actor-backend");

        let exit_reason = loop {
            let task_next = async {
                if self.tasks.is_empty() {
                    std::future::pending().await
                } else {
                    self.tasks.next().await
                }
            };

            if let Err(exit_reason) = tokio::select! {
                biased;

                sys_msg_recv = self.sys_msg_rx.recv() =>
                    self.handle_sys_msg(sys_msg_recv).await,
                call_msg = self.calls_r.recv() =>
                    self.handle_call_msg(call_msg).await,
                message_recv = self.messages_rx.recv() =>
                    self.handle_message_recv(message_recv).await,
                task_ready = task_next =>
                    if let Some(message) = task_ready.flatten() {
                        self.handle_message_recv(Some(message)).await
                    } else {
                        Ok(())
                    },
            } {
                break exit_reason
            }
        };
        tracing::trace!("exiting: {}", exit_reason.pp());

        self.sys_msg_rx.close();
        self.messages_rx.close();

        self.exit_handler.on_actor_exit(self.actor_id, exit_reason.to_owned());

        self.notify_linked_actors(exit_reason.to_owned()).await;

        while let Some(sys_msg) = self.sys_msg_rx.recv().await {
            self.handle_sys_msg_on_shutdown(sys_msg, exit_reason.to_owned()).await
        }

        tracing::trace!("exited");

        exit_reason
    }

    #[tracing::instrument(skip_all)]
    async fn handle_sys_msg(&mut self, sys_msg_recv: Option<SysMsg>) -> Result<(), Exit> {
        let sys_msg = sys_msg_recv.ok_or(BackendFailure::RxClosed("sys-msg"))?;
        tracing::trace!("[received sys-msg: {:?}", sys_msg);

        match sys_msg {
            SysMsg::SigExit(terminated, exit_reason) =>
                self.handle_sys_msg_sig_exit(terminated, exit_reason).await,
            SysMsg::Link(link_to) => self.handle_sys_msg_link(link_to).await,
            SysMsg::Unlink(unlink_from) => self.handle_sys_msg_unlink(unlink_from).await,
            SysMsg::GetInfo(report_to) => self.handle_sys_msg_get_info(report_to).await,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn handle_sys_msg_on_shutdown(&mut self, sys_msg: SysMsg, exit_reason: Exit) {
        tracing::trace!("received sys-msg when shutting down: {:?}", sys_msg);
        match sys_msg {
            SysMsg::Link(linked) =>
                if exit_reason.is_normal() {
                    self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
                } else {
                    self.send_sys_msg(linked, SysMsg::SigExit(self.actor_id, exit_reason)).await;
                },

            SysMsg::GetInfo(report_to) => {
                let _ = self.handle_sys_msg_get_info(report_to).await;
            },
            SysMsg::Unlink { .. } => (),
            SysMsg::SigExit { .. } => (),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn handle_call_msg(&mut self, call_msg: CallMsg<Message>) -> Result<(), Exit> {
        match call_msg {
            CallMsg::Exit(exit_reason) => Err(exit_reason),
            CallMsg::Link(link_to) => self.handle_call_link(link_to).await,
            CallMsg::Unlink(unlink_from) => self.handle_call_unlink(unlink_from).await,
            CallMsg::TrapExit(trap_exit) => self.handle_set_trap_exit(trap_exit),
            CallMsg::SpawnJob(fut) => self.handle_spawn_job(fut),
        }
    }

    fn handle_spawn_job(
        &mut self,
        fut: Pin<Box<dyn Future<Output = Option<Message>> + Send + Sync + 'static>>,
    ) -> Result<(), Exit> {
        self.tasks.push(fut);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn handle_message_recv(&mut self, message_recv: Option<Message>) -> Result<(), Exit> {
        let message = message_recv.ok_or(BackendFailure::RxClosed("messages"))?;
        self.inbox_w
            .send(message)
            .await
            .map_err(|_rejected| BackendFailure::InboxFull("messages"))?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn handle_sys_msg_get_info(
        &self,
        report_to: oneshot::Sender<ActorInfo>,
    ) -> Result<(), Exit> {
        let info = ActorInfo {
            actor_id: self.actor_id,

            behaviour: self.actor_type_info.0,
            args_type: self.actor_type_info.1,
            message_type: self.actor_type_info.2,

            m_queue_len: self.inbox_w.len().await,
            s_queue_len: self.signals_w.len().await,
            c_queue_len: self.calls_r.len().await,
            tasks_count: self.tasks.len(),
            trap_exit: self.watches.trap_exit,
            links: self.watches.links.iter().copied().collect(),
        };
        let _ = report_to.send(info);
        Ok(())
    }
}
