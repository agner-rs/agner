use std::time::Duration;

use agner_actors::{Context, ExitReason, System};
use futures::future;

mod common;

#[test]
fn waits_work() {
    async fn actor_behaviour(_context: &mut Context<std::convert::Infallible>, _: ()) {
        std::future::pending().await
    }

    common::run(async {
        let system = System::new(Default::default());
        let actor = system.spawn(actor_behaviour, (), Default::default()).await.unwrap();

        let mut waits = vec![];

        for _ in 0..10 {
            let mut w = Box::pin(system.wait(actor));
            tokio::select! {
                _ = w.as_mut() => unreachable!(),
                _ = tokio::time::sleep(Duration::from_millis(10)) => ()
            };
            waits.push(w);
        }

        for i in 0..5 {
            let mut w = Box::pin(system.wait(actor));
            tokio::select! {
                _ = w.as_mut() => unreachable!(),
                _ = tokio::time::sleep(Duration::from_millis(10)) => ()
            };
            waits[i * 2] = w;
        }

        waits.truncate(6);

        system.exit(actor, Default::default()).await;

        assert!(future::join_all(waits)
            .await
            .into_iter()
            .all(|e| matches!(*e, ExitReason::Normal)));
    });
}
