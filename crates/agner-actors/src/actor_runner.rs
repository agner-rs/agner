use std::future::Future;
use std::pin::Pin;

use agner_utils::std_error_pp::StdErrorPP;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};

use crate::actor::{Actor, IntoExitReason};
use crate::actor_id::ActorID;
use crate::context::{Context, Signal};
use crate::exit_reason::ExitReason;
use crate::spawn_opts::SpawnOpts;
use crate::system::SystemOpt;

pub(crate) mod call_msg;
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
    pub system_opt: SystemOpt,
    pub messages_rx: mpsc::UnboundedReceiver<Message>,
    pub sys_msg_rx: mpsc::UnboundedReceiver<SysMsg>,
    pub sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    pub spawn_opts: SpawnOpts,
}

impl<Message> ActorRunner<Message>
where
    Message: Unpin,
{
    pub async fn run<Behaviour, Arg>(self, behaviour: Behaviour, arg: Arg)
    where
        for<'a> Behaviour: Actor<'a, Arg, Message>,
    {
        let Self { actor_id, system_opt, messages_rx, sys_msg_rx, sys_msg_tx, mut spawn_opts } =
            self;

        log::trace!(
            "[{}] init [m-inbox: {:?}, s-inbox: {:?}, msg-type: {}]",
            actor_id,
            spawn_opts.msg_inbox_size(),
            spawn_opts.sig_inbox_size(),
            std::any::type_name::<Message>()
        );

        let (inbox_w, inbox_r) = pipe::new::<Message>(spawn_opts.msg_inbox_size());
        let (signals_w, signals_r) = pipe::new::<Signal>(spawn_opts.sig_inbox_size());
        let (calls_w, calls_r) = pipe::new::<CallMsg<Message>>(1);
        let mut context = Context::new(
            actor_id,
            system_opt.to_owned(),
            inbox_r,
            signals_r,
            calls_w,
            spawn_opts.take_init_ack(),
        );

        let behaviour_running = async move {
            let exit_reason = behaviour.run(&mut context, arg).await.into_exit_reason();
            context.exit(exit_reason).await;
            unreachable!()
        };

        let mut actor_backend =
            Backend {
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
                    Pin<Box<dyn Future<Output = Message> + Send + Sync + 'static>>,
                >::new(),
            };

        for link_to in spawn_opts.links() {
            actor_backend.do_link(link_to).await;
        }

        let actor_backend_running = actor_backend.run_actor_backend();

        log::trace!("[{}] running", self.actor_id);
        tokio::select! {
            _ = behaviour_running => unreachable!("Future<Output = Infallible> as returned"),
            () = actor_backend_running => (),
        }

        if let Some(system) = system_opt.rc_upgrade() {
            log::trace!("[{}] cleaning up actor-entry...", self.actor_id);
            system.actor_entry_remove(actor_id).await;
        }
    }
}

struct Backend<Message> {
    actor_id: ActorID,
    system_opt: SystemOpt,
    sys_msg_rx: mpsc::UnboundedReceiver<SysMsg>,
    sys_msg_tx: mpsc::UnboundedSender<SysMsg>,
    messages_rx: mpsc::UnboundedReceiver<Message>,
    inbox_w: PipeTx<Message>,
    signals_w: PipeTx<Signal>,
    calls_r: PipeRx<CallMsg<Message>>,
    watches: Watches,
    tasks: FuturesUnordered<Pin<Box<dyn Future<Output = Message> + Send + Sync + 'static>>>,
}

impl<Message> Backend<Message>
where
    Message: Unpin,
{
    async fn run_actor_backend(mut self) {
        log::trace!("[{}] running actor-backend", self.actor_id);

        let exit_reason = loop {
            let task_next = async {
                if self.tasks.is_empty() {
                    std::future::pending().await
                } else {
                    self.tasks.next().await
                }
            };

            if let Err(exit_reason) = tokio::select! {
                sys_msg_recv = self.sys_msg_rx.recv() =>
                    self.handle_sys_msg(sys_msg_recv).await,
                call_msg = self.calls_r.recv() =>
                    self.handle_call_msg(call_msg).await,
                message_recv = self.messages_rx.recv() =>
                    self.handle_message_recv(message_recv).await,
                task_ready = task_next =>
                    self.handle_message_recv(task_ready).await,
            } {
                break exit_reason
            }
        };
        log::trace!("[{}] exiting: {}", self.actor_id, exit_reason.pp());

        self.sys_msg_rx.close();
        self.messages_rx.close();

        self.notify_linked_actors(exit_reason.to_owned()).await;
        self.notify_waiting_chans(exit_reason.to_owned());

        while let Some(sys_msg) = self.sys_msg_rx.recv().await {
            self.handle_sys_msg_on_shutdown(sys_msg, exit_reason.to_owned()).await
        }

        log::trace!("[{}] exited", self.actor_id)
    }

    async fn handle_sys_msg(&mut self, sys_msg_recv: Option<SysMsg>) -> Result<(), ExitReason> {
        match sys_msg_recv {
            None => Err(ExitReason::RxClosed("sys-msg")),
            Some(SysMsg::SigExit(terminated, exit_reason)) =>
                self.handle_sys_msg_sig_exit(terminated, exit_reason).await,
            Some(SysMsg::Link(link_to)) => self.handle_sys_msg_link(link_to).await,
            Some(SysMsg::Unlink(unlink_from)) => self.handle_sys_msg_unlink(unlink_from).await,
            Some(SysMsg::Wait(report_to)) => self.handle_sys_msg_wait(report_to),
            Some(SysMsg::GetInfo(report_to)) => self.handle_sys_msg_get_info(report_to).await,
        }
    }

    async fn handle_sys_msg_on_shutdown(&mut self, sys_msg: SysMsg, exit_reason: ExitReason) {
        match sys_msg {
            SysMsg::Link(linked) =>
                if matches!(exit_reason, ExitReason::Normal) {
                    self.send_sys_msg(linked, SysMsg::Unlink(self.actor_id)).await;
                } else {
                    self.send_sys_msg(linked, SysMsg::SigExit(self.actor_id, exit_reason)).await;
                },
            SysMsg::Wait(report_to) => {
                let _ = report_to.send(exit_reason);
            },
            SysMsg::GetInfo(report_to) => {
                let _ = self.handle_sys_msg_get_info(report_to).await;
            },
            SysMsg::Unlink { .. } => (),
            SysMsg::SigExit { .. } => (),
        }
    }

    async fn handle_call_msg(&mut self, call_msg: CallMsg<Message>) -> Result<(), ExitReason> {
        match call_msg {
            CallMsg::Exit(exit_reason) => Err(exit_reason),
            CallMsg::Link(link_to) => self.handle_call_link(link_to).await,
            CallMsg::Unlink(unlink_from) => self.handle_call_unlink(unlink_from).await,
            CallMsg::TrapExit(trap_exit) => self.handle_set_trap_exit(trap_exit),
            CallMsg::FutureToInbox(fut) => self.handle_future_to_inbox(fut),
        }
    }

    fn handle_future_to_inbox(
        &mut self,
        fut: Pin<Box<dyn Future<Output = Message> + Send + Sync + 'static>>,
    ) -> Result<(), ExitReason> {
        self.tasks.push(fut);
        Ok(())
    }

    async fn handle_message_recv(
        &mut self,
        message_recv: Option<Message>,
    ) -> Result<(), ExitReason> {
        let message = message_recv.ok_or_else(|| ExitReason::RxClosed("messages"))?;
        self.inbox_w
            .send(message)
            .await
            .map_err(|_rejected| ExitReason::InboxFull("messages"))?;
        Ok(())
    }

    async fn handle_sys_msg_get_info(
        &self,
        report_to: oneshot::Sender<ActorInfo>,
    ) -> Result<(), ExitReason> {
        let info = ActorInfo {
            actor_id: self.actor_id,
            m_queue_len: self.inbox_w.len().await,
            s_queue_len: self.signals_w.len().await,
            c_queue_len: self.calls_r.len().await,
            tasks_count: self.tasks.len(),
            trap_exit: self.watches.trap_exit,
            links: self.watches.links.iter().copied().collect(),
            waits_len: self.watches.waits.len(),
        };
        let _ = report_to.send(info);
        Ok(())
    }
}
