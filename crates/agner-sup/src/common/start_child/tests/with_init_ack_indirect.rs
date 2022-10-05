use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{Event, Exit, Signal, System};
use agner_test_actor::{TestActor, TestActorRegistry};
use futures::future;

use crate::common::start_child::{self, WithAck};
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

    let worker =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        WithAck::new()
            .with_init_timeout(INIT_TIMEOUT)
            .with_stop_timeout(INIT_CANCEL_TIMEOUT)
            .into(),
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let intermediary = async {
        let intermediary_id = init_rx.await.unwrap();
        let intermediary = registry.lookup::<Infallible>(intermediary_id).await.unwrap();
        intermediary.set_link(worker.actor_id(), true).await;
        intermediary.init_ack(Some(worker.actor_id())).await;
        intermediary
    };

    let (start_child_result, intermediary) = future::join(start_child_result, intermediary).await;
    assert_eq!(start_child_result.ok(), Some(worker.actor_id()));

    intermediary.exit(Exit::normal()).await;
    assert!(intermediary.wait().await.is_normal());

    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());

    worker.exit(Exit::shutdown()).await;
    assert!(worker.wait().await.is_shutdown());

    let event = sup.next_event(SMALL_TIMEOUT).await.unwrap();
    if let Event::Signal(Signal::Exit(exited_id, exit_reason)) = event {
        assert_eq!(exited_id, worker.actor_id());
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

    let worker =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        WithAck::new()
            .with_init_timeout(INIT_TIMEOUT)
            .with_stop_timeout(INIT_CANCEL_TIMEOUT)
            .into(),
        [s1.to_owned(), s2.to_owned()],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let intermediary = async {
        let intermediary_id = init_rx.await.unwrap();
        let intermediary = registry.lookup::<Infallible>(intermediary_id).await.unwrap();
        intermediary.set_link(worker.actor_id(), true).await;
        intermediary.init_ack(Some(worker.actor_id())).await;
        intermediary
    };

    let (start_child_result, intermediary) = future::join(start_child_result, intermediary).await;
    assert_eq!(start_child_result.ok(), Some(worker.actor_id()));

    assert_eq!(s1.resolve(), Some(worker.actor_id()));
    assert_eq!(s2.resolve(), Some(worker.actor_id()));

    intermediary.exit(Exit::normal()).await;
    assert!(intermediary.wait().await.is_normal());
    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());

    assert_eq!(s1.resolve(), Some(worker.actor_id()));
    assert_eq!(s2.resolve(), Some(worker.actor_id()));

    worker.exit(Exit::shutdown()).await;
    assert!(worker.wait().await.is_shutdown());

    assert!(s1.resolve().is_none());
    assert!(s2.resolve().is_none());

    let event = sup.next_event(SMALL_TIMEOUT).await.unwrap();
    if let Event::Signal(Signal::Exit(exited_id, exit_reason)) = event {
        assert_eq!(exited_id, worker.actor_id());
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

    let worker =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        WithAck::new()
            .with_init_timeout(INIT_TIMEOUT)
            .with_stop_timeout(INIT_CANCEL_TIMEOUT)
            .into(),
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned());

    let intermediary = async {
        let intermediary_id = init_rx.await.unwrap();
        let intermediary = registry.lookup::<Infallible>(intermediary_id).await.unwrap();
        intermediary.set_link(worker.actor_id(), true).await;
        intermediary
    };

    let (start_child_result, intermediary) = future::join(start_child_result, intermediary).await;
    assert!(start_child_result.is_err());
    assert!(intermediary.wait().await.is_shutdown());
    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());
    assert!(worker.wait().await.is_linked());

    assert!(sup.next_event(SMALL_TIMEOUT).await.is_none());
}
