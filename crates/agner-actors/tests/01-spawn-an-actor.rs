use std::convert::Infallible;

use agner_actors::{Context, Exit, System};
use tokio::sync::oneshot;

mod common;

#[test]
fn actor_returning_unit() {
    async fn actor_behaviour(_context: &mut Context<Infallible>, arg: oneshot::Sender<()>) {
        arg.send(()).expect("oneshot send error");
    }

    common::run(async {
        let system = System::new(Default::default());
        let (tx, rx) = oneshot::channel();
        let _actor = system
            .spawn(actor_behaviour, tx, Default::default())
            .await
            .expect("Failed to start an actor");
        rx.await.expect("oneshot recv closed");
    });
}

#[test]
fn actor_returning_exit_reason() {
    async fn actor_behaviour(_context: &mut Context<Infallible>, arg: oneshot::Sender<()>) -> Exit {
        arg.send(()).expect("oneshot send error");
        Exit::shutdown()
    }

    common::run(async {
        let system = System::new(Default::default());
        let (tx, rx) = oneshot::channel();
        let _actor = system
            .spawn(actor_behaviour, tx, Default::default())
            .await
            .expect("Failed to start an actor");
        rx.await.expect("oneshot recv closed");
    });
}

#[test]
fn actor_returning_never() {
    async fn actor_behaviour(
        context: &mut Context<Infallible>,
        arg: oneshot::Sender<()>,
    ) -> std::convert::Infallible {
        arg.send(()).expect("oneshot send error");
        context.exit(Default::default()).await
    }

    common::run(async {
        let system = System::new(Default::default());
        let (tx, rx) = oneshot::channel();
        let _actor = system
            .spawn(actor_behaviour, tx, Default::default())
            .await
            .expect("Failed to start an actor");
        rx.await.expect("oneshot recv closed");
    });
}
