#![cfg(test)]
use std::time::Duration;

use agner_actors::{Context, ExitReason, System, SystemConfig};

#[tokio::test]
#[ignore]
async fn test() {
	let _ = dotenv::dotenv();
	let _ = pretty_env_logger::try_init_timed();

	let system_config = SystemConfig { ..Default::default() };
	let system = System::new(system_config);

	async fn actor_behaviour(ctx: &mut Context<()>, arg: &'static str) -> ExitReason {
		eprintln!("[{}] actor_behaviour({:?})", ctx.actor_id(), arg);
		ctx.exit(Default::default()).await;
		unreachable!()
	}

	let actor_id = system
		.spawn(actor_behaviour, "this is an argument", Default::default())
		.await
		.expect("Failed to start actor");
	eprintln!("actor-id: {}", actor_id);

	tokio::time::sleep(Duration::from_secs(1)).await;
}
