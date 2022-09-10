use agner_actor::oneshot;
use futures::StreamExt;
use std::time::{Duration, Instant};

use agner_actor::System;
use sample_actors::echo_actor::*;

const MSGS_PER_ACTOR: usize = 10_000;
const ACTORS_COUNT: usize = 2_000;
const MAX_CONCURRENT_ACTORS: usize = 100;

#[test]
fn main() {
	pretty_env_logger::init_timed();

	let runtime = tokio::runtime::Runtime::new().expect("Failed to create Runtime");
	runtime.block_on(main_impl());
	let t0 = Instant::now();
	runtime.shutdown_timeout(Duration::from_secs(3));
	log::info!("Took {:?} to shutdown", t0.elapsed());
}

async fn main_impl() {
	let system = agner_sys1::system::SystemOne::create(Default::default());

	futures::stream::iter(
		(0..ACTORS_COUNT).map(|actor_id| run_one_actor(system.to_owned(), actor_id)),
	)
	.buffer_unordered(MAX_CONCURRENT_ACTORS)
	.for_each(|()| async {})
	.await;

	tokio::time::sleep(Duration::from_secs(1)).await;
}

async fn run_one_actor(system: impl System, id: usize) {
	let echo_actor = system
		.start::<EchoActor>(EchoArgs { id }.into(), Default::default())
		.expect("Failed to start actor");

	log::info!("started {:?}: {}", id, echo_actor);

	let mut responses = vec![];

	for rq_id in 0..MSGS_PER_ACTOR {
		let (tx, rx) = oneshot::channel();
		let echo_request = EchoRequest { rq_id, reply_to: tx };
		system.send(echo_actor, echo_request);
		responses.push((rx, id, rq_id));
	}

	for (rx, actor_id, rq_id) in responses {
		let echo_response = rx.await.expect("oneshot rx-failure");

		assert_eq!(echo_response.actor, actor_id);
		assert_eq!(echo_response.rq_id, rq_id);
	}

	log::info!("shutting down {:?}: {}", id, echo_actor);
	system.stop(echo_actor, Default::default());
}
