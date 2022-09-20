use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{Context, System, SystemConfig};

mod common;

#[test]
fn hit_small_system_limit() {
    common::run(hit_system_limit(5));
}

#[test]
#[ignore]
fn hit_large_system_limit() {
    common::run(hit_system_limit(1_000_000));
}

async fn hit_system_limit(max_actors: usize) {
    async fn actor_behaviour(context: &mut Context<Infallible>, _arg: usize) {
        loop {
            let event = context.next_event().await;
            log::info!("event received: {:?}", event);
        }
    }

    let system = System::new(SystemConfig { max_actors, ..Default::default() });

    for i in 0..max_actors {
        assert!(system.spawn(actor_behaviour, i, Default::default()).await.is_ok());
    }

    assert!(system.spawn(actor_behaviour, max_actors, Default::default()).await.is_err());

    tokio::time::sleep(Duration::from_secs(1)).await;
}
