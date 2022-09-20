#![cfg(feature = "test-actor")]

use agner::actors::{Event, Exit, Signal, SpawnOpts};
use agner::test_actor::{TestActor, TestActorRegistry};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use agner::utils::std_error_pp::StdErrorPP;
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
