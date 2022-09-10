use std::time::Duration;

use agner_actor::error::ExitReason;
use agner_actor::System;

use crate::common::basic::*;

pub async fn run(system: impl System) {
	let actor_id = 0;
	let actor = system
		.start::<BasicActor>(BasicArgs::from(actor_id).into(), Default::default())
		.expect("Failed to start an actor");

	for rq_id in 0..1_000 {
		let (rq, rs) = BasicRequest::create(rq_id, Duration::from_millis(1));
		system.send(actor, rq);

		let rs = rs.await.expect("Received no response");
		assert_eq!(rs.rq_id, rq_id);
		assert_eq!(rs.actor_id, actor_id);
	}

	let actor_exited = system.wait(actor);
	system.stop(actor, Default::default());
	assert!(matches!(actor_exited.await, ExitReason::Normal));
}
