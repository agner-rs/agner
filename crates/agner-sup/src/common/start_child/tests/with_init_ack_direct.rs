use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{Event, Exit, Signal, System};
use agner_test_actor::{TestActor, TestActorRegistry};
use futures::future;

use crate::common::start_child::{self, InitType};
use crate::Service;

const SMALL_TIMEOUT: Duration = Duration::from_millis(100);
const INIT_TIMEOUT: Duration = Duration::from_secs(1);
const INIT_CANCEL_TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test]
async fn start_child_with_ack_no_regs() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let sup =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    sup.set_trap_exit(true).await;

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::WithAck { init_timeout: INIT_TIMEOUT, stop_timeout: INIT_CANCEL_TIMEOUT },
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let child_init_acked = async {
        let child_id = init_rx.await.unwrap();
        let child = registry.lookup::<Infallible>(child_id).await.unwrap();
        child.init_ack(Default::default()).await;
        child
    };

    let (start_child_result, child) = future::join(start_child_result, child_init_acked).await;
    assert_eq!(start_child_result.ok(), Some(child.actor_id()));

    child.exit(Exit::shutdown()).await;
    assert!(child.wait().await.is_shutdown());

    let event = sup.next_event(SMALL_TIMEOUT).await.unwrap();
    if let Event::Signal(Signal::Exit(exited_id, exit_reason)) = event {
        assert_eq!(exited_id, child.actor_id());
        assert!(exit_reason.is_shutdown());
    } else {
        panic!("Received unexpected event: {:?}", event);
    }
}

#[tokio::test]
async fn start_child_with_ack_with_regs() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let s1 = Service::new();
    let s2 = Service::new();

    assert!(s1.resolve().is_none());
    assert!(s2.resolve().is_none());

    let sup =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    sup.set_trap_exit(true).await;

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::WithAck { init_timeout: INIT_TIMEOUT, stop_timeout: INIT_CANCEL_TIMEOUT },
        [s1.to_owned(), s2.to_owned()],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let child_init_acked = async {
        let child_id = init_rx.await.unwrap();
        let child = registry.lookup::<Infallible>(child_id).await.unwrap();
        child.init_ack(Default::default()).await;
        child
    };

    let (start_child_result, child) = future::join(start_child_result, child_init_acked).await;
    assert_eq!(start_child_result.ok(), Some(child.actor_id()));

    assert_eq!(s1.resolve(), Some(child.actor_id()));
    assert_eq!(s2.resolve(), Some(child.actor_id()));

    child.exit(Exit::shutdown()).await;
    assert!(child.wait().await.is_shutdown());

    assert!(s1.resolve().is_none());
    assert!(s2.resolve().is_none());

    let event = sup.next_event(SMALL_TIMEOUT).await.unwrap();
    if let Event::Signal(Signal::Exit(exited_id, exit_reason)) = event {
        assert_eq!(exited_id, child.actor_id());
        assert!(exit_reason.is_shutdown());
    } else {
        panic!("Received unexpected event: {:?}", event);
    }
}

#[tokio::test]
async fn start_child_with_ack_timeout_shutdown() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let sup =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    sup.set_trap_exit(true).await;

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::WithAck { init_timeout: INIT_TIMEOUT, stop_timeout: INIT_CANCEL_TIMEOUT },
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let child = async {
        let child_id = init_rx.await.unwrap();
        registry.lookup::<Infallible>(child_id).await.unwrap()
    };

    let (start_child_result, child) = future::join(start_child_result, child).await;

    assert!(start_child_result.is_err());
    assert!(child.wait().await.is_shutdown());
    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());
}

#[tokio::test]
async fn start_child_with_ack_timeout_killed() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let sup =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    sup.set_trap_exit(true).await;

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::WithAck { init_timeout: INIT_TIMEOUT, stop_timeout: INIT_CANCEL_TIMEOUT },
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let child = async {
        let child_id = init_rx.await.unwrap();
        let child = registry.lookup::<Infallible>(child_id).await.unwrap();
        child.set_trap_exit(true).await;
        child
    };

    let (start_child_result, child) = future::join(start_child_result, child).await;

    assert!(start_child_result.is_err());
    assert!(child.wait().await.is_kill());
    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());
}
