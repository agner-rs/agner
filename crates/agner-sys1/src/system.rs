use std::collections::VecDeque;
use std::hash::Hasher;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use tokio::sync::mpsc;

use agner_actor::error::{ExitError, ExitReason, WellKnownReason};
use agner_actor::{oneshot, Actor, ActorID, Message, StartOpts};

use agner_actor::System;

mod fmt_debug;

mod config;
pub use config::SystemOneConfig;

mod entry_cleanup_guard;
use entry_cleanup_guard::EntryCleanupGuard;

mod all_actors;

use crate::actor_entry::ActorEntry;
use crate::error::SystemOneError;
use crate::sys_msg::SysMsg;

#[derive(Clone)]
pub struct SystemOne(Arc<SystemOneImpl>);

impl SystemOne {
	pub fn create(config: SystemOneConfig) -> Self {
		let ids_pool = Mutex::new((0, (0..config.max_actors).collect()));
		let actors = (0..config.max_actors).map(|_| None).map(RwLock::new).collect();

		let system_id = {
			use std::hash::Hash;
			let mut h = std::collections::hash_map::DefaultHasher::new();
			std::time::Instant::now().hash(&mut h);
			h.finish() as usize
		};

		let inner = SystemOneImpl { system_id, config, ids_pool, actors };
		let inner = Arc::new(inner);

		Self(inner)
	}

	pub fn config(&self) -> &SystemOneConfig {
		&self.0.config
	}

	pub fn send_sys(&self, to: ActorID, sys_msg: SysMsg) -> bool {
		self.entry_read_existing(to, |entry| entry.tx_sys().send(sys_msg).is_ok())
			.unwrap_or(false)
	}
}

impl SystemOne {
	fn entry_read_existing<Ret, F: FnOnce(&dyn ActorEntry) -> Ret>(
		&self,
		actor_id: ActorID,
		f: F,
	) -> Option<Ret> {
		let this = self.0.as_ref();

		let (system_id, actor_idx, requested_serial_id) = actor_id.into();

		if system_id != this.system_id {
			return None
		}

		// log::trace!("SystemOne: r-locking [{}] BEFORE", actor_id);
		let slot = this.actors[actor_idx].read().expect("Failed to r-lock `actors`");
		// log::trace!("SystemOne: r-locking [{}] AFTER", actor_id);

		slot.as_ref()
			.filter(|entry| requested_serial_id == entry.serial_id())
			.map(|e| f(e.as_ref()))
	}
	fn entry_write<Ret, F: FnOnce(&mut Option<Box<dyn ActorEntry>>) -> Ret>(
		&self,
		actor_id: ActorID,
		f: F,
	) -> Option<Ret> {
		let this = self.0.as_ref();

		let (system_id, actor_idx, requested_serial_id) = actor_id.into();

		if system_id != this.system_id {
			return None
		}

		// log::trace!("SystemOne: w-locking [{}] BEFORE", actor_id);
		let mut slot = this.actors[actor_idx].write().expect("Failed to w-lock `actors`");
		// log::trace!("SystemOne: w-locking [{}] AFTER", actor_id);

		if slot.as_ref().map(|e| e.serial_id() == requested_serial_id).unwrap_or(true) {
			Some(f(&mut slot))
		} else {
			None
		}
	}

	fn all_processes<'a>(&'a self) -> impl Iterator<Item = ActorID> + 'a {
		self.0.actors.iter().enumerate().filter_map(|(actor_idx, slot)| {
			slot.read()
				.expect("failed to r-lock `actors`")
				.as_ref()
				.map(|entry| (self.0.system_id, actor_idx, entry.serial_id()).into())
		})
	}
}

pub(super) struct SystemOneImpl {
	system_id: usize,
	config: SystemOneConfig,
	ids_pool: Mutex<(usize, VecDeque<usize>)>,
	actors: Arc<[RwLock<Option<Box<dyn ActorEntry>>>]>,
}

#[async_trait::async_trait]
impl System for SystemOne {
	type Error = SystemOneError;

	fn start<A: Actor<Self>>(
		&self,
		seed: A::Seed,
		start_opts: StartOpts<A::Seed, A::Message>,
	) -> Result<ActorID, Self::Error> {
		let this = self.0.as_ref();

		let (actor_idx, serial_id) = {
			let mut ids_pool = this.ids_pool.lock().expect("Failed to lock `ids_pool`");

			let serial_id = ids_pool.0;
			ids_pool.0 += 1;

			let actor_idx = ids_pool.1.pop_front().ok_or(SystemOneError::ActorsCountLimit)?;
			assert!(actor_idx < this.actors.len());

			(actor_idx, serial_id)
		};

		let actor_id = ActorID::from((self.0.system_id, actor_idx, serial_id));

		let (tx_msg, rx_msg) = mpsc::unbounded_channel();
		let (tx_sys, rx_sys) = mpsc::unbounded_channel();

		let entry = crate::actor_entry::create::<A>(serial_id, tx_msg, tx_sys.to_owned());

		let _ = tokio::spawn({
			let system = self.to_owned();
			async move {
				let cleanup_guard = EntryCleanupGuard { system: system.to_owned(), actor_id };
				if let Err(reason) = crate::actor_runner::run::<A>(
					system,
					actor_id,
					start_opts.link_with,
					start_opts.seed_heir,
					start_opts.inbox_heir,
					start_opts.inbox,
					seed,
					tx_sys,
					rx_sys,
					rx_msg,
					cleanup_guard,
				)
				.await
				{
					log::error!("ActorRunner failure [id: {:?}]: {}", actor_id, reason);
				}
			}
		});

		let mut slot = this.actors[actor_idx].write().expect("Failed to w-lock `actors`");
		*slot = Some(entry);

		Ok(actor_id)
	}

	fn send<M: Message>(&self, to: ActorID, message: M) {
		// log::trace!("SystemOne::send [to: {}] BEFORE", to);
		let _found = self
			.entry_read_existing(to, |entry| {
				if let Some(tx) = entry.tx_msg().downcast_ref::<mpsc::UnboundedSender<M>>() {
					let _ = tx.send(message);
				}
			})
			.is_some();
		// log::trace!("System::send [to: {}; found: {}] AFTER", to, _found);
	}

	fn stop(&self, actor_id: ActorID, exit_reason: ExitReason) {
		self.entry_read_existing(actor_id, |e| {
			let _ = e.tx_sys().send(SysMsg::Shutdown(exit_reason));
		});
	}

	async fn wait(&self, actor_id: ActorID) -> ExitReason {
		let (tx, rx) = oneshot::channel();
		self.send_sys(actor_id, SysMsg::Wait(tx));
		rx.await
			.ok_or_else(|| WellKnownReason::NoActor(actor_id))
			.map_err(ExitError::from)
			.map_err(ExitReason::from)
			.unwrap_or_else(|exit_reason| exit_reason)
	}

	async fn shutdown(&self, timeout: Duration) {
		log::info!("System shutdown requested");

		// clear ids pool, so no new processes will be created
		self.0.ids_pool.lock().expect("Failed to w-lock `ids_pool`").1.clear();

		let killed_and_waited = futures::future::join_all(self.all_processes().map(|actor_id| {
			self.stop(actor_id, ExitReason::shutdown());
			self.wait(actor_id)
		}));

		if let Ok(exit_reasons) = tokio::time::timeout(timeout, killed_and_waited).await {
			log::info!("System shutdown complete ({} actors terminated)", exit_reasons.len());
		} else {
			log::warn!("Timed out on waiting for the system to shutdown");
		}
	}
}
