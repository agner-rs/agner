use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use agner_actor::error::{ExitError, ExitReason, WellKnownReason};
use agner_actor::{oneshot, ActorID, Context as ActorContext, Message, Signal};
use tokio::sync::mpsc;

use agner_actor::Actor;

mod error;
pub use error::ActorRunnerError;

mod context;
use context::Context;

mod internals;
use internals::Internals;

use crate::sys_msg::SysMsg;
use crate::system::SystemOne;

pub(crate) async fn run<A: Actor<SystemOne>>(
	system: SystemOne,
	actor_id: ActorID,
	link_with: impl IntoIterator<Item = ActorID>,
	seed_heir: Option<oneshot::Tx<A::Seed>>,
	inbox_heir: Option<oneshot::Tx<Vec<A::Message>>>,
	inbox_messages: impl IntoIterator<Item = A::Message>,
	mut seed: A::Seed,
	tx_sys: mpsc::UnboundedSender<SysMsg>,
	mut rx_sys: mpsc::UnboundedReceiver<SysMsg>,
	mut rx_msg: mpsc::UnboundedReceiver<A::Message>,
	_entry_cleanup_guard: impl Sized,
) -> Result<(), ActorRunnerError> {
	log::trace!("[{}|{}] starting", actor_id, std::any::type_name::<A>());

	let mut context = Context::<A>::create(&mut seed, system.to_owned(), actor_id, tx_sys);
	let mut internals = Internals::<A::Message>::create(actor_id, inbox_messages);

	log::trace!("[{}|{}] linking", actor_id, std::any::type_name::<A>());
	for linked_actor_id in link_with {
		context.link(linked_actor_id);
		internals.link_add(linked_actor_id);
	}
	log::trace!("[{}|{}] running actor lifecycle", actor_id, std::any::type_name::<A>());

	let exit_reason =
		run_actor_lifecycle(&mut context, &mut internals, actor_id, &mut rx_msg, &mut rx_sys)
			.await?;

	log::trace!("[{}|{}] actor terminated", actor_id, std::any::type_name::<A>());

	log::trace!("[{}|{}] processing remaining sys-messages", actor_id, std::any::type_name::<A>());
	rx_sys.close();
	while let Some(sys_msg) = rx_sys.recv().await {
		match sys_msg {
			SysMsg::Link(id) => {
				system.send_sys(
					id,
					SysMsg::Exit(actor_id, WellKnownReason::NoActor(actor_id).into()),
				);
			},
			SysMsg::Unlink(id) => {
				internals.link_rm(id);
			},
			SysMsg::Wait(reply_to) => {
				let _ = reply_to.send(exit_reason.to_owned());
			},
			ignored => log::trace!("Ignoring sys-msg on shutdown: {:?}", ignored),
		}
	}

	log::trace!("[{}|{}] notifying linked actors", actor_id, std::any::type_name::<A>());
	match &exit_reason {
		ExitReason::Normal => internals.linked().for_each(|linked_actor_id| {
			system.send_sys(linked_actor_id, SysMsg::Unlink(actor_id));
		}),
		ExitReason::Error(exit_error) => internals.linked().for_each(|linked_actor_id| {
			system.send_sys(linked_actor_id, SysMsg::Exit(actor_id, exit_error.to_owned()));
		}),
	}

	log::trace!("[{}|{}] notifying waiting actors", actor_id, std::any::type_name::<A>());
	for waiting in internals.waiting_drain() {
		let _ = waiting.send(exit_reason.to_owned());
	}

	log::trace!(
		"[{}|{}] sending the seed to the heir ({})",
		actor_id,
		std::any::type_name::<A>(),
		seed_heir.is_some()
	);
	if let Some(seed_heir) = seed_heir {
		seed_heir.send(seed);
	}

	log::trace!(
		"[{}|{}] sending the inbox to the heir ({})",
		actor_id,
		std::any::type_name::<A>(),
		inbox_heir.is_some()
	);
	if let Some(inbox_heir) = inbox_heir {
		inbox_heir.send(internals.inbox_drain().collect())
	}
	log::trace!("[{}|{}] done", actor_id, std::any::type_name::<A>());

	Ok(())
}

fn is_kill(exit_reason: &ExitReason) -> bool {
	if let ExitReason::Error(ExitError::WellKnown(well_known)) = exit_reason {
		matches!(well_known.as_ref(), &WellKnownReason::Kill)
	} else {
		false
	}
}

fn process_task_requests<'a, A: Actor<SystemOne>>(
	actor_id: ActorID,
	context: &mut Context<'a, A>,
	internals: &mut Internals<A::Message>,
) {
	internals.tasks_rm(context.tasks_to_stop_drain().inspect(|task_id| {
		log::trace!("[{}|{}] terminating {}", actor_id, std::any::type_name::<A>(), task_id)
	}));
	internals.tasks_add(context.tasks_to_start_drain().inspect(|(task_id, _)| {
		log::trace!("[{}|{}] starting {}", actor_id, std::any::type_name::<A>(), task_id)
	}));
}

async fn run_actor_lifecycle<'a, A: Actor<SystemOne>>(
	context: &mut Context<'a, A>,
	internals: &mut Internals<A::Message>,
	actor_id: ActorID,
	rx_msg: &mut mpsc::UnboundedReceiver<A::Message>,
	rx_sys: &mut mpsc::UnboundedReceiver<SysMsg>,
) -> Result<ExitReason, ActorRunnerError> {
	log::trace!("[{}|{}] invoking Actor::init", actor_id, std::any::type_name::<A>());

	let actor_init_result = run_actor_init(actor_id, context, internals, rx_sys).await?;

	log::trace!(
		"[{}|{}] Actor::init returned (is_err: {})",
		actor_id,
		std::any::type_name::<A>(),
		actor_init_result.is_err()
	);

	let mut actor = match actor_init_result {
		Ok(actor) => actor,
		Err(exit_reason) => return Ok(exit_reason),
	};

	process_task_requests(actor_id, context, internals);

	log::trace!("[{}|{}] entering actor loop", actor_id, std::any::type_name::<A>());
	let exit_reason =
		run_actor_loop(&mut actor, context, internals, actor_id, rx_msg, rx_sys).await?;

	if !is_kill(&exit_reason) {
		log::trace!(
			"[{}|{}] actor exited (will invoke Actor::terminate) [reason: {}]",
			actor_id,
			std::any::type_name::<A>(),
			exit_reason
		);
		if let Err(_) = tokio::time::timeout(
			context.system().config().actor_termination_timeout,
			actor.terminated(context, exit_reason.to_owned()),
		)
		.await
		{
			log::warn!(
				"[{}|{}] took too long to gracefully shutdown",
				actor_id,
				std::any::type_name::<A>(),
			);
		}
	} else {
		log::trace!(
			"[{}|{}] actor killed (will not invoke Actor::terminate)",
			actor_id,
			std::any::type_name::<A>()
		);
	}

	Ok(exit_reason)
}

async fn run_actor_init<'a, A: Actor<SystemOne>>(
	actor_id: ActorID,
	context: &mut Context<'a, A>,
	internals: &mut Internals<A::Message>,
	rx_sys: &mut mpsc::UnboundedReceiver<SysMsg>,
) -> Result<Result<A, ExitReason>, ActorRunnerError> {
	let mut actor_init_fut = A::init(context);
	let mut actor_init_fut_ref = actor_init_fut.as_mut();

	let actor_init_result = loop {
		if let Some(signal) = internals.signal_deq() {
			break Err(ExitReason::well_known(WellKnownReason::UnhandledSignal(Arc::new(signal))))
		}

		tokio::select! {
			actor_init_result = actor_init_fut_ref => {
				match actor_init_result {
					Ok(actor) => break Ok(actor),
					Err(exit_reason) => break Err(exit_reason),
				};
			},

			sys = rx_sys.recv() => {
				actor_init_fut_ref = actor_init_fut.as_mut();
				let sys = sys.ok_or(ActorRunnerError::RxClosed("sys"))?;
				handle_sys_msg(actor_id, STATE_NAME_INIT, internals, sys).await?;
			}
		}
	};

	Ok(actor_init_result)
}

async fn run_actor_loop<'a, A: Actor<SystemOne>>(
	actor: &mut A,
	context: &mut Context<'a, A>,
	internals: &mut Internals<A::Message>,
	actor_id: ActorID,
	rx_msg: &mut mpsc::UnboundedReceiver<A::Message>,
	rx_sys: &mut mpsc::UnboundedReceiver<SysMsg>,
) -> Result<ExitReason, ActorRunnerError> {
	let mut state = Running::Ready(actor, context);

	loop {
		if let Some(exit_reason) = internals.exit_requested() {
			log::trace!(
				"[{}|{}] exit requested [reason: {}]",
				actor_id,
				std::any::type_name::<A>(),
				exit_reason,
			);
			return Ok(exit_reason.to_owned())
		}

		let on_ready = |internals: &mut Internals<A::Message>, fut_output| {
			let (actor, context, result) = fut_output;
			let result: Result<(), ExitReason> = result;
			let context: &mut Context<_> = context;

			process_task_requests(actor_id, context, internals);

			if let Err(exit_reason) = result {
				return Err(Ok(exit_reason))
			}

			if let Some(sig) = internals.signal_deq() {
				let actor_fut = Box::pin(invoke_handle_sig(actor, context, sig));
				Ok(Running::HandleSignal(actor_fut))
			} else if let Some(msg) = internals.inbox_deq() {
				let actor_fut = Box::pin(invoke_handle_msg(actor, context, msg));
				Ok(Running::HandleMessage(actor_fut))
			} else {
				Ok(Running::Ready(actor, context))
			}
		};

		match state {
			Running::Ready(actor, context) =>
				if let Some(signal) = internals.signal_deq() {
					let actor_fut = Box::pin(invoke_handle_sig(actor, context, signal));
					state = Running::HandleSignal(actor_fut);
				} else {
					let selected = select_once(
						STATE_NAME_READY,
						actor_id,
						internals,
						rx_msg,
						rx_sys,
						Box::pin(std::future::pending()),
						|_internals, ()| unreachable!(),
						|internals, _| on_ready(internals, (actor, context, Ok(()))),
					)
					.await;

					match selected {
						Ok(state_next) => state = state_next,
						Err(ret) => return ret,
					}
				},
			Running::HandleSignal(hs_fut) => {
				let selected = select_once(
					STATE_NAME_BUSY_HS,
					actor_id,
					internals,
					rx_msg,
					rx_sys,
					hs_fut,
					on_ready,
					|_internals, hs_fut| Ok(Running::HandleSignal(hs_fut)),
				)
				.await;

				match selected {
					Ok(state_next) => state = state_next,
					Err(ret) => return ret,
				}
			},
			Running::HandleMessage(hm_fut) => {
				let selected = select_once(
					STATE_NAME_BUSY_HM,
					actor_id,
					internals,
					rx_msg,
					rx_sys,
					hm_fut,
					on_ready,
					|_internals, hm_fut| Ok(Running::HandleMessage(hm_fut)),
				)
				.await;

				match selected {
					Ok(state_next) => state = state_next,
					Err(ret) => return ret,
				}
			},
		}
	}
}

async fn select_once<M, S, F, HR, HB>(
	state_name: &str,
	actor_id: ActorID,
	internals: &mut Internals<M>,
	rx_msg: &mut mpsc::UnboundedReceiver<M>,
	rx_sys: &mut mpsc::UnboundedReceiver<SysMsg>,
	mut busy_fut: Pin<Box<F>>,
	on_ready: HR,
	on_busy: HB,
) -> Result<S, Result<ExitReason, ActorRunnerError>>
where
	M: Message,
	F: Future,
	HR: FnOnce(&mut Internals<M>, F::Output) -> Result<S, Result<ExitReason, ActorRunnerError>>,
	HB: FnOnce(&mut Internals<M>, Pin<Box<F>>) -> Result<S, Result<ExitReason, ActorRunnerError>>,
{
	let busy_fut_ref = busy_fut.as_mut();
	tokio::select! {
		() = internals.tasks_poll() => on_busy(internals, busy_fut),
		ready = busy_fut_ref => on_ready(internals, ready),
		msg = rx_msg.recv() => {
			log::trace!(
				"[{}|??] received message while busy",
				actor_id,
			);
			let msg = msg.ok_or(ActorRunnerError::RxClosed("msg")).map_err(Err)?;
			internals.inbox_enq(msg).map_err(|_rejected_msg| ActorRunnerError::InboxFull).map_err(Err)?;
			on_busy(internals, busy_fut)
		}
		sys = rx_sys.recv() => {
			log::trace!(
				"[{}|??] received sys-msg while busy",
				actor_id,
			);

			let sys = sys.ok_or(ActorRunnerError::RxClosed("sys")).map_err(Err)?;
			handle_sys_msg(actor_id, state_name, internals, sys).await.map_err(Err)?;
			on_busy(internals, busy_fut)
		}
	}
}

async fn invoke_handle_msg<'a, 's, A: Actor<SystemOne>>(
	actor: &'a mut A,
	context: &'a mut Context<'s, A>,
	message: A::Message,
) -> (&'a mut A, &'a mut Context<'s, A>, Result<(), ExitReason>) {
	log::trace!(
		"[{}|{}] invoking Actor::handle_message",
		context.actor_id(),
		std::any::type_name::<A>(),
	);

	let result = actor.handle_message(context, message).await;
	(actor, context, result)
}

async fn invoke_handle_sig<'a, 's, A: Actor<SystemOne>>(
	actor: &'a mut A,
	context: &'a mut Context<'s, A>,
	signal: Signal,
) -> (&'a mut A, &'a mut Context<'s, A>, Result<(), ExitReason>) {
	log::trace!(
		"[{}|{}] invoking Actor::handle_signal",
		context.actor_id(),
		std::any::type_name::<A>(),
	);

	let result = actor.handle_signal(context, signal).await;
	(actor, context, result)
}

async fn handle_sys_msg<M>(
	actor_id: ActorID,
	state_name: &str,
	internals: &mut Internals<M>,
	sys_msg: SysMsg,
) -> Result<(), ActorRunnerError> {
	log::trace!("[{}|??] handling sys-msg [state: {}]", actor_id, state_name,);

	match sys_msg {
		SysMsg::Shutdown(reason) => internals.request_exit(reason),
		SysMsg::Wait(waiting) => internals.waiting_add(waiting),
		SysMsg::Link(id) => internals.link_add(id),
		SysMsg::Unlink(id) => internals.link_rm(id),
		SysMsg::Exit(linked_id, exit_error) =>
			internals.signal_enq(Signal::Exit(linked_id, exit_error)),
	}

	Ok(())
}

const STATE_NAME_INIT: &str = "INIT";
const STATE_NAME_READY: &str = "READY";
const STATE_NAME_BUSY_HM: &str = "BUSY:HANDLE_MESSAGE";
const STATE_NAME_BUSY_HS: &str = "BUSY:HANDLE_SIGNAL";

enum Running<A, C, HM, HS> {
	Ready(A, C),
	HandleMessage(HM),
	HandleSignal(HS),
}
