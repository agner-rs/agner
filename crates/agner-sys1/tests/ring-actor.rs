use std::time::{Duration, Instant};

use agner_actor::error::WellKnownReason;
use agner_actor::{oneshot, System};
use agner_sys1::system::SystemOneConfig;
use sample_actors::ring_actor::{EchoRequest, RingActor, RingArgs};

const RING_SIZE: usize = 20;

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
	let config = SystemOneConfig { max_actors: RING_SIZE, ..Default::default() };
	let system = agner_sys1::system::SystemOne::create(config);

	let t0 = Instant::now();
	let actor = system
		.start::<RingActor>(RingArgs { id: RING_SIZE - 1 }.into(), Default::default())
		.expect("Failed to start the actor");
	log::info!("Took {:?} to start the ring of {:?} actors", t0.elapsed(), RING_SIZE);

	let (tx, rx) = oneshot::channel();
	let t0 = Instant::now();
	system.send(actor, EchoRequest { rq_id: 0, reply_to: tx });
	let response = rx.await;
	log::info!("Took {:?} to receive the response {:?}", t0.elapsed(), response);

	system.stop(actor, WellKnownReason::Shutdown.into());

	tokio::time::sleep(Duration::from_secs(10)).await;
}
