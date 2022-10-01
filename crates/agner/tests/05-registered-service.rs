use std::time::Duration;

use agner::actors::ActorID;
use agner::sup::Service;

#[macro_use]
mod common;

agner_test!(registered_service, async {
    use crate::*;

    let reg = Service::new();

    let resolving = (0..100_000)
        .map(|n| Duration::from_micros(n * 10))
        .map(|delay| (delay, reg.to_owned()))
        .map(|(delay, reg)| async move {
            tokio::time::sleep(delay).await;
            let resolution = reg.wait().await;
            resolution
        });
    let resolving = futures::future::join_all(resolving);

    let registering = async move {
        for i in (0..usize::MAX).cycle() {
            let actor_id: ActorID = format!("{}.{}.{}", i, i, i).parse().unwrap();
            let _registered = reg.register(actor_id).await;
        }
    };

    let resolved = tokio::select! {
        resolved = resolving => resolved,
        _ = registering => unreachable!()
    };

    for (idx, result) in resolved.into_iter().enumerate() {
        assert!(result.is_some(), "{}", idx)
    }
});
