use std::time::{Duration, Instant};

use agner_actor::System;
use sample_actors::actor_with_tasks::ActorWithTasks;

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

	let actor = system
		.start::<ActorWithTasks>(().into(), Default::default())
		.expect("Failed to start actor");

	let _exit_reason = system.wait(actor).await;
}
