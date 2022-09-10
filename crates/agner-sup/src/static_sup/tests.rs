use std::fmt;
use std::time::Duration;

use agner_actor::{Actor, ActorID, Context, Seed, System};
use agner_sys1::system::{SystemOne, SystemOneConfig};
use futures::{stream, StreamExt};

use super::hlist::HListLen;
use super::*;

fn make_system(max_actors: usize) -> SystemOne {
	let config = SystemOneConfig { max_actors, ..Default::default() };

	SystemOne::create(config)
}

#[allow(unused)]
struct Dummy<T>(T);

#[async_trait::async_trait]
impl<T: Seed + fmt::Debug, Sys: System> Actor<Sys> for Dummy<T> {
	type Seed = T;
	type Message = ExitReason;

	async fn init<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		log::info!(
			"Dummy<{}>[{}] init [seed: {:?}]",
			std::any::type_name::<T>(),
			context.actor_id(),
			context.seed()
		);
		Ok(Self(context.seed_mut().take()))
	}

	async fn handle_message<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		context.exit(message).await;
		unreachable!()
	}

	async fn terminated<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		self,
		context: &mut Ctx,
		exit_reason: ExitReason,
	) {
		log::info!(
			"Dummy<{}>[{}] terminated [reason: {:?}]",
			std::any::type_name::<T>(),
			context.actor_id(),
			exit_reason
		);
	}
}

#[tokio::test]
async fn ergonomics() {
	let _ = pretty_env_logger::try_init_timed();

	use child_spec::SupSpec;

	let sup_spec = SupSpec::new(restart_strategy::GiveUp)
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("one"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("two"));

	log::info!("sup_spec: {:#?}", sup_spec);

	let system = make_system(10);

	let sup = system
		.start::<StaticSup<_, _, _, _>>(sup_spec.into(), Default::default())
		.expect("Failed to start actor");

	tokio::time::sleep(Duration::from_secs(1)).await;

	system.stop(sup, ExitReason::shutdown());
	let sup_exit_reason = system.wait(sup).await;
	log::info!("sup terminated: {:?}", sup_exit_reason);
}

#[tokio::test]
async fn strategy_give_up_child_failure_terminates_sup() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(10);

	let sup_spec = SupSpec::new(restart_strategy::GiveUp)
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("one"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("two"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("three"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("four"));

	for iteration in 0..sup_spec.children.len() {
		log::info!("Iteration: {:?}", iteration);

		let sup = system
			.start::<StaticSup<_, _, _, _>>(sup_spec.to_owned().into(), Default::default())
			.expect("Failed to start sup");
		let sup_api = StaticSupApi::from(sup);
		assert!(is_alive(&system, sup).await);

		let children = stream::iter(0..sup_spec.children.len())
			.then(|idx| {
				let system = &system;
				async move { sup_api.get_child_by_idx(system, idx).await.unwrap().unwrap() }
			})
			.collect::<Vec<_>>()
			.await;

		system.send(children[iteration], ExitReason::shutdown());

		tokio::time::sleep(Duration::from_millis(100)).await;

		for id in children.iter().copied() {
			assert!(!is_alive(&system, id).await)
		}
		assert!(!is_alive(&system, sup).await);

		system.stop(sup, ExitReason::shutdown());
		system.wait(sup).await;
	}
}

async fn is_alive(system: &SystemOne, actor_id: ActorID) -> bool {
	let exited = system.wait(actor_id);
	tokio::time::timeout(Default::default(), exited).await.is_err()
}

#[tokio::test]
async fn strategy_one_for_one_failed_child_gets_restarted_the_rest_keeps_running() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(10);

	let sup_spec = SupSpec::new(restart_strategy::OneForOne::new())
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("one"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("two"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("three"))
		.child::<Dummy<SharedSeed<&'static str>>, _>(SharedSeed::from("four"));

	for iteration in 0..sup_spec.children.len() {
		log::info!("Iteration: {:?}", iteration);

		let sup = system
			.start::<StaticSup<_, _, _, _>>(sup_spec.to_owned().into(), Default::default())
			.expect("Failed to start sup");
		let sup_api = StaticSupApi::from(sup);
		assert!(is_alive(&system, sup).await);

		let children = stream::iter(0..sup_spec.children.len())
			.then(|idx| {
				let system = &system;
				async move { sup_api.get_child_by_idx(system, idx).await.unwrap().unwrap() }
			})
			.collect::<Vec<_>>()
			.await;

		system.send(children[iteration], ExitReason::shutdown());

		tokio::time::sleep(Duration::from_millis(100)).await;

		assert!(is_alive(&system, sup).await);

		for (idx, id) in children.iter().copied().enumerate() {
			assert!(is_alive(&system, id).await ^ (idx == iteration))
		}

		let children_updated = stream::iter(0..sup_spec.children.len())
			.then(|idx| {
				let system = &system;
				async move { sup_api.get_child_by_idx(system, idx).await.unwrap().unwrap() }
			})
			.collect::<Vec<_>>()
			.await;

		assert_eq!(children.len(), children_updated.len());
		for (idx, (old, new)) in children.into_iter().zip(children_updated).enumerate() {
			assert!((old == new) ^ (idx == iteration));
		}

		system.stop(sup, ExitReason::shutdown());
		system.wait(sup).await;
	}
}
