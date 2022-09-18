use std::time::Duration;

use agner::actors::{ActorID, BoxError, Context, Event, Never, System};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use futures::future;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();
    run().await.expect("Ew...")
}

async fn run() -> Result<(), BoxError> {
    let system = System::new(Default::default());

    let p1 = system.spawn(ping_pong, "first", Default::default()).await?;
    let p2 = system.spawn(ping_pong, "second", Default::default()).await?;

    system.send(p1, Message::Init { peer: p2 }).await;

    assert!(future::join_all([system.wait(p1), system.wait(p2)])
        .timeout(Duration::from_secs(3))
        .await
        .is_err());

    system.exit(p1, Default::default()).await;
    system.exit(p2, Default::default()).await;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Init { peer: ActorID },
    Ping { from: ActorID },
    Pong { from: ActorID },
}

async fn ping_pong(context: &mut Context<Message>, name: &'static str) -> Never {
    context.init_ack(Default::default());

    let mut peer_opt = None;
    loop {
        match context.next_event().await {
            Event::Signal { .. } => unreachable!(),

            Event::Message(Message::Init { peer }) if peer_opt.is_none() => {
                peer_opt = Some(peer);
                log::info!(
                    "[{}|{}] Received an Message::Init [peer: {}]",
                    context.actor_id(),
                    name,
                    peer
                );
                context.system().send(peer, Message::Ping { from: context.actor_id() }).await;
            },
            Event::Message(Message::Init { peer }) => {
                log::warn!(
                    "[{}|{}] Received an unexpected Message::Init (already inited) [peer: {}]",
                    context.actor_id(),
                    name,
                    peer
                );
            },

            Event::Message(Message::Ping { from })
                if peer_opt.is_none() || peer_opt == Some(from) =>
            {
                log::info!(
                    "[{}|{}] Received Message::Ping [from: {}]",
                    context.actor_id(),
                    name,
                    from
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
                peer_opt = Some(from);
                context.system().send(from, Message::Pong { from: context.actor_id() }).await;
                context.system().send(from, Message::Ping { from: context.actor_id() }).await;
            },
            Event::Message(Message::Ping { from }) => {
                log::warn!(
                    "[{}|{}] Received Message::Ping from an unexpected actor: {}",
                    context.actor_id(),
                    name,
                    from
                );
            },

            Event::Message(Message::Pong { from }) if peer_opt == Some(from) => {
                log::info!(
                    "[{}|{}] Received Message::Pong [from: {}]",
                    context.actor_id(),
                    name,
                    from
                );
            },
            Event::Message(Message::Pong { from }) => {
                log::warn!(
                    "[{}|{}] Received Message::Pong from an unexpected actor: {}",
                    context.actor_id(),
                    name,
                    from
                );
            },
        }
    }
}
