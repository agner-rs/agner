use futures::{stream, StreamExt};

use agner_actor::error::ExitReason;
use agner_actor::System;

use crate::common::tree::*;

pub async fn run(system: impl System) {
	log::info!("begin");
	let spec = TreeArgs::new("/").children([
		TreeArgs::new("linked").children([TreeArgs::new("one"), TreeArgs::new("two")]),
		TreeArgs::new("orphans")
			.link(false)
			.children([TreeArgs::new("one"), TreeArgs::new("two")]),
	]);
	log::info!("spec: {:#?}", spec);

	let actor_paths: &[&[&str]] = &[
		&[],
		&["linked"],
		&["orphans"],
		&["linked", "one"],
		&["linked", "two"],
		&["orphans", "one"],
		&["orphans", "two"],
	];

	let root = system
		.start::<TreeActor>(spec.into(), Default::default())
		.expect("Failed to start an actor");

	log::info!("started [root: {}]", root);

	let actor_ids = stream::iter(actor_paths)
		.then(|&path| {
			log::info!("looking up {}|{:?}...", root, path);
			api::lookup_child(&system, root, path.into_iter().copied())
		})
		.map(|opt| opt.expect("Failed to lookup actor"))
		.collect::<Vec<_>>()
		.await;

	assert!(api::lookup_child(&system, root, ["non-existent"]).await.is_none());

	for id in actor_ids.iter().copied() {
		log::info!("about to ping {}", id);
		assert!(api::ping(&system, id).await);
	}

	let root_exited = system.wait(root);
	system.stop(root, ExitReason::shutdown());

	log::info!("waiting for root to exit");
	// Not a guarantee at all
	let root_exited = root_exited.await;
	log::info!("root exited: {}", root_exited);

	for (idx, alive) in
		[(0, false), (1, false), (2, true), (3, false), (4, false), (5, true), (6, true)]
	{
		log::info!("pinging [{}] expected to be alive — {}", idx, alive);
		assert_eq!(
			api::ping(&system, actor_ids[idx]).await,
			alive,
			"misbehaving — {:?}",
			&actor_paths[idx]
		);
	}
	log::info!("end");
}
