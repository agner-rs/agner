use std::time::Duration;

use agner_actor::error::ExitReason;
use agner_actor::{oneshot, Actor, System, UniqueSeed};

use crate::error::SystemOneError;
use crate::system::{SystemOne, SystemOneConfig};

mod dummy;
use dummy::DummyActor;

mod sup;
use sup::{Sup, SupRq};

mod wrk;
use wrk::{Wrk, WrkRq};

fn make_system(max_actors: usize) -> SystemOne {
	let config = SystemOneConfig { max_actors, ..Default::default() };
	SystemOne::create(config)
}

#[tokio::test]
async fn max_actors_limit_works() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(1);
	let actor = system
		.start::<DummyActor<(), oneshot::Tx<()>>>(().into(), Default::default())
		.expect("Failed to start actor");

	let (tx, rx) = oneshot::channel::<()>();
	system.send(actor, tx);
	assert!(rx.await.is_none());

	let err_limit_reached = system.start::<DummyActor<(), ()>>(().into(), Default::default());
	assert!(matches!(err_limit_reached, Err(SystemOneError::ActorsCountLimit)));

	let actor_terminated = system.wait(actor);
	system.stop(actor, Default::default());

	assert!(matches!(actor_terminated.await, ExitReason::Normal));

	let actor = system
		.start::<DummyActor<(), oneshot::Tx<()>>>(().into(), Default::default())
		.expect("Failed to start actor");

	let (tx, rx) = oneshot::channel::<()>();
	system.send(actor, tx);
	assert!(rx.await.is_none());

	system.shutdown(Duration::from_secs(3)).await;
}

#[tokio::test]
async fn supervision_works() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(2);
	type WorkerMsg = <Wrk as Actor<SystemOne>>::Message;
	type Supervisor = Sup<Wrk, <Wrk as Actor<SystemOne>>::Seed, <Wrk as Actor<SystemOne>>::Message>;

	let sup = system
		.start::<Supervisor>(UniqueSeed::from(0).into(), Default::default())
		.expect("Failed to start actor");

	for expected_incarnation in 0..10 {
		for expected_rq_id in 1..10 {
			let (rq, rs) = oneshot::channel();
			system.send(sup, SupRq::<WorkerMsg>::Pass(WrkRq::Rq(rq)));
			let (reported_incarnation, reported_rq_id) =
				rs.await.expect("Worker failed to respond");
			assert_eq!(reported_incarnation, expected_incarnation);
			assert_eq!(reported_rq_id, expected_rq_id);
		}

		let (rq, rs) = oneshot::channel();
		system.send(sup, SupRq::<WorkerMsg>::ChildID(rq));
		let child = rs.await.expect("Sup failed to return child-id");
		let child_done = system.wait(child);
		system.stop(child, ExitReason::shutdown());
		child_done.await;
	}

	system.shutdown(Duration::from_secs(3)).await;
}

#[tokio::test]
async fn supervision_no_service_disruption() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(2);
	type WorkerMsg = <Wrk as Actor<SystemOne>>::Message;
	type Supervisor = Sup<Wrk, <Wrk as Actor<SystemOne>>::Seed, <Wrk as Actor<SystemOne>>::Message>;

	let sup = system
		.start::<Supervisor>(UniqueSeed::from(0).into(), Default::default())
		.expect("Failed to start actor");

	let client_working = async {
		loop {
			let (rq, rs) = oneshot::channel();
			system.send(sup, SupRq::<WorkerMsg>::Pass(WrkRq::Rq(rq)));
			let (_reported_incarnation, _reported_rq_id) =
				rs.await.expect("Worker failed to respond");
		}
	};

	let killer_working = async {
		loop {
			tokio::time::sleep(Duration::from_millis(100)).await;
			system.send(sup, SupRq::<WorkerMsg>::Pass(WrkRq::Exit));
		}
	};

	let five_seconds = tokio::time::sleep(Duration::from_secs(5));

	tokio::select! {
		_elapsed = five_seconds => {},
		_ = client_working => unreachable!(),
		_ = killer_working => unreachable!(),
	};
}
