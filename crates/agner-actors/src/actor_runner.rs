use std::sync::Arc;

use tokio::sync::mpsc;

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
		let Self { actor_id, system_opt, messages_rx, sys_msg_rx, sys_msg_tx, spawn_opts } = self;

		log::trace!(
			"[{}] init [m-inbox: {:?}, s-inbox: {:?}, msg-type: {}]",
			actor_id,
			spawn_opts.msg_inbox_size(),
			spawn_opts.sig_inbox_size(),
			std::any::type_name::<Message>()
		);

		let (inbox_w, inbox_r) = pipe::new::<Message>(spawn_opts.msg_inbox_size());
		let (signals_w, signals_r) = pipe::new::<Signal>(spawn_opts.sig_inbox_size());
		let (calls_w, calls_r) = pipe::new::<CallMsg>(1);
		let mut context =
			Context::new(actor_id, system_opt.to_owned(), inbox_r, signals_r, calls_w);

		let behaviour_running = async move {
			let exit_reason = behaviour.run(&mut context, arg).await.into_exit_reason();
			context.exit(exit_reason).await;
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
	calls_r: PipeRx<CallMsg>,
	watches: Watches,
}

impl<Message> Backend<Message>
where
	Message: Unpin,
{
	async fn run_actor_backend(mut self) {
		log::trace!("[{}] running actor-backend", self.actor_id);
		let exit_reason = loop {
			if let Err(exit_reason) = tokio::select! {
				sys_msg_recv = self.sys_msg_rx.recv() =>
					self.handle_sys_msg(sys_msg_recv).await,
				call_msg = self.calls_r.recv() =>
					self.handle_call_msg(call_msg).await,
				message_recv = self.messages_rx.recv() =>
					self.handle_message_recv(message_recv).await,
			} {
				break exit_reason
			}
		};
		log::trace!("[{}] exiting: {}", self.actor_id, exit_reason);

		let exit_reason = Arc::new(exit_reason);

		self.sys_msg_rx.close();
		self.messages_rx.close();

		self.notify_linked_actors(exit_reason.to_owned()).await;

		while let Some(sys_msg) = self.sys_msg_rx.recv().await {
			self.handle_sys_msg_on_shutdown(sys_msg, exit_reason.to_owned()).await
		}

		log::trace!("[{}] exited", self.actor_id)
	}

	async fn handle_sys_msg(&mut self, sys_msg_recv: Option<SysMsg>) -> Result<(), ExitReason> {
		match sys_msg_recv {
			None => Err(ExitReason::RxClosed("sys-msg")),
			Some(SysMsg::Exit(exit_reason)) => Err(exit_reason),
			Some(SysMsg::Exited(terminated, exit_reason)) =>
				self.handle_sys_msg_exit(terminated, exit_reason).await,
			Some(SysMsg::Link(link_to)) => self.handle_sys_msg_link(link_to).await,
			Some(SysMsg::Unlink(unlink_from)) => self.handle_sys_msg_unlink(unlink_from).await,
		}
	}

	async fn handle_sys_msg_on_shutdown(&mut self, sys_msg: SysMsg, exit_reason: Arc<ExitReason>) {
		match sys_msg {
			SysMsg::Link(linked) =>
				if !matches!(*exit_reason, ExitReason::Normal) {
					self.send_sys_msg(linked, SysMsg::Exited(self.actor_id, exit_reason)).await;
				},
			SysMsg::Unlink { .. } => (),
			SysMsg::Exited { .. } => (),
			SysMsg::Exit { .. } => (),
		}
	}

	async fn handle_call_msg(&mut self, call_msg: CallMsg) -> Result<(), ExitReason> {
		match call_msg {
			CallMsg::Exit(exit_reason) => Err(exit_reason),
			CallMsg::Link(link_to) => self.handle_call_link(link_to).await,
			CallMsg::Unlink(unlink_from) => self.handle_call_unlink(unlink_from).await,
			CallMsg::TrapExit(trap_exit) => self.handle_set_trap_exit(trap_exit),
		}
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
}
