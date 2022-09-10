#![cfg(test)]

use agner_actor::System;

use std::time::Duration;

const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

mod util;

#[tokio::test]
async fn sc01_basic() {
	let _ = pretty_env_logger::try_init_timed();
	let system = util::make_system(10);
	log::info!("system: {:?}", system);
	test_scenarios::sc01_basic::run(system.to_owned()).await;
	system.shutdown(GRACEFUL_SHUTDOWN_TIMEOUT).await;
}

#[tokio::test]
async fn sc02_a_tree() {
	let _ = pretty_env_logger::try_init_timed();
	let system = util::make_system(10);
	log::info!("system: {:?}", system);
	test_scenarios::sc02_a_tree::run(system.to_owned()).await;
	system.shutdown(GRACEFUL_SHUTDOWN_TIMEOUT).await;
}
