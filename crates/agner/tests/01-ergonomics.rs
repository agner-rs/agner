use agner_actors::{Context, Exit, SpawnOpts};
use futures::future;
use tokio::sync::oneshot;

#[macro_use]
mod common;

agner_test!(ergonomics, async move {
    use crate::*;

    enum Message {
        Ping(oneshot::Sender<()>),
        Exit(Exit),
    }
    async fn an_actor(context: &mut Context<Message>, _args: ()) {
        loop {
            match context.next_message().await {
                Message::Ping(reply_to) => {
                    let _ = reply_to.send(());
                },
                Message::Exit(reason) => {
                    context.exit(reason).await;
                },
            }
        }
    }

    let system = crate::common::system(5);

    let a1 = system
        .spawn(an_actor, (), SpawnOpts::new())
        .await
        .expect("Failed to start an actor");

    let (tx, rx) = oneshot::channel();
    system.send(a1, Message::Ping(tx)).await;
    rx.await.expect("oneshot-rx failure");

    let exit_requested = system.send(a1, Message::Exit(Exit::shutdown()));
    let a1_exited = system.wait(a1);

    let (_, a1_exit_reason) = future::join(exit_requested, a1_exited).await;
    assert!(a1_exit_reason.is_shutdown(), "{}", a1_exit_reason);
});
