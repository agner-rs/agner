use std::time::Duration;

use agner_actors::{ActorID, Context, Event, ExitReason, Signal, System};
use futures::{stream, StreamExt, TryStreamExt};
use tokio::sync::oneshot;

mod common;

#[test]
fn links_test() {
	#[derive(Debug)]
	enum Request {
		Link(ActorID),
		Unlink(ActorID),
		Ping,
		Exit(ExitReason),
		TrapExit(bool),
	}

	async fn actor_behaviour(
		context: &mut Context<(Request, oneshot::Sender<()>)>,
		_arg: (),
	) -> ExitReason {
		loop {
			match context.next_event().await {
				Event::Message((request, reply_to)) => {
					let () = match request {
						Request::Exit(reason) => {
							let _ = reply_to.send(());
							break reason
						},
						Request::Link(to) => context.link(to).await,
						Request::Unlink(from) => context.unlink(from).await,
						Request::TrapExit(trap_exit) => context.trap_exit(trap_exit).await,
						Request::Ping => (),
					};
					let _ = reply_to.send(());
				},

				Event::Signal(Signal::Exited(terminated, reason)) =>
					log::info!("[{}] {} has exited: {}", context.actor_id(), terminated, reason),
			}
		}
	}

	common::run(async {
		let system = System::new(Default::default());

		let call = |a: ActorID, rq: Request| {
			let system = &system;
			async move {
				let (tx, rx) = oneshot::channel::<()>();
				system.send(a, (rq, tx)).await;
				rx.await
			}
		};

		let a = stream::iter(0..=8)
			.then(|_| system.spawn(actor_behaviour, (), Default::default()))
			.try_collect::<Vec<_>>()
			.await
			.unwrap();

		// 0: The one to exit
		// 1: Never linked
		// 2: 2 link 0
		call(a[2], Request::Link(a[0])).await.unwrap();
		// 3: 0 link 3
		call(a[0], Request::Link(a[3])).await.unwrap();

		// 4: 4 link 0, 4 unlink 0
		call(a[4], Request::Link(a[0])).await.unwrap();
		call(a[4], Request::Unlink(a[0])).await.unwrap();

		// 5: 5 link 0, 0 unlink 5
		call(a[5], Request::Link(a[0])).await.unwrap();
		call(a[0], Request::Unlink(a[5])).await.unwrap();

		// 6: 6 link 0
		call(a[6], Request::Link(a[0])).await.unwrap();
		// 7: 7 link 6
		call(a[7], Request::Link(a[6])).await.unwrap();
		// 8: trap-exit=true, 8 link 7
		call(a[8], Request::TrapExit(true)).await.unwrap();
		call(a[8], Request::Link(a[7])).await.unwrap();

		for actor in a.iter().copied() {
			assert!(call(actor, Request::Ping).await.is_ok());
		}

		assert!(call(a[0], Request::Exit(ExitReason::Shutdown(None))).await.is_ok());

		log::info!("---");
		tokio::time::sleep(Duration::from_millis(500)).await;

		for &(idx, expected_ok) in &[
			(0, false),
			(1, true),
			(2, false),
			(3, false),
			(4, true),
			(5, true),
			(6, false),
			(7, false),
			(8, true),
		] {
			assert_eq!(call(a[idx], Request::Ping).await.is_ok(), expected_ok, "[{}]", idx);
		}

		log::info!("---");
		tokio::time::sleep(Duration::from_millis(100)).await;

		for actor in a.iter().copied() {
			let _ = call(actor, Request::Exit(ExitReason::Shutdown(None))).await;
		}

		log::info!("---");
		tokio::time::sleep(Duration::from_millis(100)).await;
	})
}
