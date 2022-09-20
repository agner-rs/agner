#![cfg(feature = "test-actor")]

use agner::actors::{Exit, SpawnOpts};
use agner::test_actor::{TestActor, TestActorRegistry};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use std::collections::HashSet;
use std::convert::Infallible;

#[macro_use]
mod common;

agner_test!(actor_link_with_spawn_opts, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    let a2 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a1.actor_id()),
    )
    .await
    .unwrap();

    let a1_info = system.actor_info(a1.actor_id()).await.unwrap();
    let a2_info = system.actor_info(a2.actor_id()).await.unwrap();

    assert_eq!(a1_info.links.as_ref(), &[a2.actor_id()]);
    assert_eq!(a2_info.links.as_ref(), &[a1.actor_id()]);
});

agner_test!(actor_link_with_context_link, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    let a2 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    let a1_info = system.actor_info(a1.actor_id()).await.unwrap();
    let a2_info = system.actor_info(a2.actor_id()).await.unwrap();
    assert_eq!(a1_info.links.as_ref(), &[]);
    assert_eq!(a2_info.links.as_ref(), &[]);

    a1.set_link(a2.actor_id(), true).await;

    let a1_info = system.actor_info(a1.actor_id()).await.unwrap();
    let a2_info = system.actor_info(a2.actor_id()).await.unwrap();

    assert_eq!(a1_info.links.as_ref(), &[a2.actor_id()]);
    assert_eq!(a2_info.links.as_ref(), &[a1.actor_id()]);
});

agner_test!(linked_actors_terminates_together, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    let a2 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a1.actor_id()),
    )
    .await
    .unwrap();
    let a3 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a2.actor_id()),
    )
    .await
    .unwrap();

    assert_eq!(system.actor_info(a1.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);
    assert_eq!(
        system
            .actor_info(a2.actor_id())
            .await
            .unwrap()
            .links
            .as_ref()
            .into_iter()
            .copied()
            .collect::<HashSet<_>>(),
        [a1.actor_id(), a3.actor_id()].into_iter().collect::<HashSet<_>>()
    );
    assert_eq!(system.actor_info(a3.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);

    a1.exit(Exit::shutdown()).await;

    assert!(a1.wait().await.is_shutdown());
    assert!(a2.wait().await.is_linked());
    assert!(a3.wait().await.is_linked());
});

agner_test!(link_does_not_trigger_on_exit_normal, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    let a2 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a1.actor_id()),
    )
    .await
    .unwrap();
    let a3 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a2.actor_id()),
    )
    .await
    .unwrap();

    assert_eq!(system.actor_info(a1.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);
    assert_eq!(
        system
            .actor_info(a2.actor_id())
            .await
            .unwrap()
            .links
            .as_ref()
            .into_iter()
            .copied()
            .collect::<HashSet<_>>(),
        [a1.actor_id(), a3.actor_id()].into_iter().collect::<HashSet<_>>()
    );
    assert_eq!(system.actor_info(a3.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);

    a1.exit(Exit::normal()).await;

    assert!(a1.wait().await.is_normal());
    assert!(a2.wait().timeout(common::SMALL_TIMEOUT).await.is_err());
    assert!(a3.wait().timeout(common::SMALL_TIMEOUT).await.is_err());
});

agner_test!(unlink_works, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    let a2 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a1.actor_id()),
    )
    .await
    .unwrap();
    let a3 = TestActor::<Infallible>::start(
        registry.to_owned(),
        system.to_owned(),
        SpawnOpts::new().with_link(a2.actor_id()),
    )
    .await
    .unwrap();

    assert_eq!(system.actor_info(a1.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);
    assert_eq!(
        system
            .actor_info(a2.actor_id())
            .await
            .unwrap()
            .links
            .as_ref()
            .into_iter()
            .copied()
            .collect::<HashSet<_>>(),
        [a1.actor_id(), a3.actor_id()].into_iter().collect::<HashSet<_>>()
    );
    assert_eq!(system.actor_info(a3.actor_id()).await.unwrap().links.as_ref(), &[a2.actor_id()]);

    a3.set_link(a2.actor_id(), false).await;

    a1.exit(Exit::shutdown()).await;

    assert!(a1.wait().await.is_shutdown());
    assert!(a2.wait().await.is_linked());
    assert!(a3.wait().timeout(common::SMALL_TIMEOUT).await.is_err());
});
