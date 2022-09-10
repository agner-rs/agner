use agner_actor::error::ExitReason;
use agner_actor::{oneshot, Actor, Context, NonEmptySeed, SharedSeed, System, UniqueSeed};
use agner_sys1::system::{SystemOne, SystemOneConfig};

use super::{DynamicSup, StartError, SupRq};

fn make_system(max_actors: usize) -> SystemOne {
	let config = SystemOneConfig { max_actors, ..Default::default() };

	SystemOne::create(config)
}

#[allow(unused)]
struct Dummy {
	a: usize,
	b: usize,
}

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for Dummy {
	type Seed = SharedSeed<(usize, usize)>;
	type Message = std::convert::Infallible;

	async fn init<Ctx: Context<System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		let &(a, b) = context.seed().value();
		Ok(Self { a, b })
	}
}

#[tokio::test]
async fn basic_test() {
	let _ = pretty_env_logger::try_init_timed();

	let system = make_system(5);
	let mut counter: usize = 0;
	let wsf = move |rq: usize| {
		counter += 1;
		SharedSeed::from((counter, rq))
	};
	let sup = system
		.start::<DynamicSup<Dummy, _, usize>>(UniqueSeed::from(wsf), Default::default())
		.expect("Failed to spawn sup");

	log::info!("sup: {}", sup);

	let mut workers = vec![];

	for idx in 1 as usize..=4 {
		log::info!("starting worker #{}", idx);
		let (tx, rx) = oneshot::channel();
		let start_child_rq = SupRq::StartChild(idx, tx);
		system.send(sup, start_child_rq);
		let worker_id = rx.await.expect("No reply from sup").expect("Failed to spawn worker");
		log::info!("  started: {}", worker_id);
		workers.push(worker_id);
	}

	let (tx, rx) = oneshot::channel();
	let start_child_rq = SupRq::StartChild(5 as usize, tx);
	system.send(sup, start_child_rq);
	assert!(matches!(rx.await.expect("No reply from sup"), Err(StartError::SystemError(_))));

	while let Some(worker_id) = workers.pop() {
		log::info!("about to stop {}", worker_id);
		let (tx, rx) = oneshot::channel();
		let stop_child_rq = SupRq::<usize>::StopChild(worker_id, tx);
		system.send(sup, stop_child_rq);
		assert!(matches!(rx.await.expect("No reply from sup"), Ok(())));
	}
}
