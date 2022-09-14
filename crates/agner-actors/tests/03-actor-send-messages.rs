use std::sync::Arc;
use std::time::{Duration, Instant};

use agner_actors::{ActorID, Context, Event, System, SystemConfig};
use tokio::sync::{oneshot, Mutex};

mod common;

#[test]
fn send_single_compatible_message() {
	let output = Arc::new(Mutex::new(Vec::<(&'static str, &'static str)>::new()));
	async fn actor_behaviour(
		context: &mut Context<&'static str>,
		(actor_name, output): (&'static str, Arc<Mutex<Vec<(&'static str, &'static str)>>>),
	) {
		loop {
			if let Event::Message(message) = context.next_event().await {
				output.lock().await.push((actor_name, message));
			}
		}
	}

	common::run(async move {
		let system = System::new(Default::default());

		let one = system
			.spawn(actor_behaviour, ("one", output.to_owned()))
			.await
			.expect("Failed to start an actor");
		let two = system
			.spawn(actor_behaviour, ("two", output.to_owned()))
			.await
			.expect("Failed to start an actor");
		let three = system
			.spawn(actor_behaviour, ("three", output.to_owned()))
			.await
			.expect("Failed to start an actor");

		system.send(one, "hello One").await;
		system.send(two, "hi Mr.Two").await;
		system.send(three, "are you Three?").await;

		tokio::time::sleep(Duration::from_secs(1)).await;

		let mut expected =
			[("one", "hello One"), ("two", "hi Mr.Two"), ("three", "are you Three?")].to_vec();
		let mut actual = output.lock().await;

		expected.sort();
		actual.sort();

		assert_eq!(&expected[..], &actual[..]);
	})
}

#[test]
fn echo_actor_via_send() {
	async fn actor_behaviour(context: &mut Context<(usize, oneshot::Sender<usize>)>, _arg: ()) {
		loop {
			if let Event::Message((id, reply_to)) = context.next_event().await {
				let _ = reply_to.send(id);
			}
		}
	}

	common::run(async {
		let system = System::new(Default::default());
		let echo_actor = system.spawn(actor_behaviour, ()).await.expect("Failed to start an actor");

		for i in 0usize..1_000_000 {
			let (tx, rx) = oneshot::channel::<usize>();
			system.send(echo_actor, (i, tx)).await;
			assert_eq!(i, rx.await.expect("oneshot rx error"));
		}
	})
}

#[test]
fn echo_actor_via_chan() {
	async fn actor_behaviour(context: &mut Context<(usize, oneshot::Sender<usize>)>, _arg: ()) {
		loop {
			if let Event::Message((id, reply_to)) = context.next_event().await {
				let _ = reply_to.send(id);
			}
		}
	}

	common::run(async {
		let system = System::new(Default::default());
		let echo_actor = system.spawn(actor_behaviour, ()).await.expect("Failed to start an actor");
		let echo_actor_tx = system
			.channel::<(usize, oneshot::Sender<usize>)>(echo_actor)
			.await
			.expect("Failed to obtain actor's tx-chan");

		for i in 0usize..1_000_000 {
			let (tx, rx) = oneshot::channel::<usize>();
			echo_actor_tx.send((i, tx)).expect("mpsc tx failure");
			assert_eq!(i, rx.await.expect("oneshot rx error"));
		}
	})
}

#[test]
fn small_ring() {
	common::run(actor_ring(10));
}

#[test]
fn large_ring() {
	common::run(actor_ring(100_000));
}

async fn actor_ring(ring_size: usize) {
	async fn actor_behaviour(
		context: &mut Context<(usize, oneshot::Sender<usize>)>,
		forward_to: Option<ActorID>,
	) {
		loop {
			if let Event::Message((id, reply_to)) = context.next_event().await {
				if let Some(forward_to) = forward_to {
					context.system().send(forward_to, (id, reply_to)).await;
				} else {
					let _ = reply_to.send(id);
				}
			}
		}
	}

	let system = System::new(SystemConfig { max_actors: ring_size, ..Default::default() });

	let mut prev = None;
	for _i in 0..ring_size {
		let actor = system
			.spawn(actor_behaviour, prev)
			.await
			.expect("Failed to start another actor");
		prev = Some(actor);
	}

	if let Some(last) = prev {
		for i in 1usize..10 {
			let (tx, rx) = oneshot::channel::<usize>();
			let t0 = Instant::now();
			system.send(last, (i, tx)).await;
			rx.await.expect("oneshot rx error");
			log::info!("ring-size: {} [{}] RTT: {:?}", ring_size, i, t0.elapsed());
		}
	}
}
