#![cfg(feature = "test-actor")]

use agner::actors::{Event, Exit, Signal};
use agner::test_actor::{TestActor, TestActorRegistry};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use agner::utils::std_error_pp::StdErrorPP;
use std::convert::Infallible;

#[macro_use]
mod common;

agner_test!(actor_invokes_context_exit_and_then_exits, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = crate::common::system(common::SMALL_SYSTEM_SIZE);

    let a1 = TestActor::<Infallible>::start(registry, system.to_owned(), Default::default())
        .await
        .unwrap();
    a1.exit(Default::default()).await;
    let reason = a1.wait().timeout(common::EXIT_TIMEOUT).await.unwrap();

    assert!(reason.is_normal(), "{}", reason.pp());
});

agner_test!(actor_is_terminated_via_system_exit, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = crate::common::system(common::SMALL_SYSTEM_SIZE);

    let a1 = TestActor::<Infallible>::start(registry, system.to_owned(), Default::default())
        .await
        .unwrap();

    system.exit(a1.actor_id(), Default::default()).await;
    let reason = a1.wait().timeout(common::EXIT_TIMEOUT).await.unwrap();
    assert!(reason.is_normal(), "{}", reason.pp());
});

agner_test!(actor_with_trapexit_is_terminated_via_system_exit_receives_signal, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = crate::common::system(common::SMALL_SYSTEM_SIZE);

    let a1 = TestActor::<usize>::start(registry, system.to_owned(), Default::default())
        .await
        .unwrap();

    a1.set_trap_exit(true).await;

    a1.post_message(1).await;
    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Ok(Some(Event::Message(1)))));

    system.exit(a1.actor_id(), Exit::normal()).await;
    a1.post_message(2).await;

    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Ok(Some(Event::Signal(Signal::Exit(_, _))))));

    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Ok(Some(Event::Message(2)))));

    system.exit(a1.actor_id(), Exit::shutdown()).await;
    a1.post_message(3).await;

    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Ok(Some(Event::Signal(Signal::Exit(_, _))))));

    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Ok(Some(Event::Message(3)))));

    system.exit(a1.actor_id(), Exit::kill()).await;
    let received = a1.next_event(common::SMALL_TIMEOUT).timeout(common::SMALL_TIMEOUT * 2).await;
    assert!(matches!(received, Err(_) | Ok(None)));
});
