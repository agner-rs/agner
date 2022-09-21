use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{Event, Exit, Signal, System};
use agner_test_actor::{TestActor, TestActorRegistry};

use crate::common::start_child::{self, InitType, StartChild};
use crate::Service;

const SMALL_TIMEOUT: Duration = Duration::from_millis(100);

#[tokio::test]
async fn start_child_no_ack_no_regs() {
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
    let start_child = start_child::without_ack(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::NoAck,
        [],
    );

    let child = {
        let child_id = start_child.start_child(system.to_owned(), ()).await.unwrap();
        let _ = init_rx.await;
        registry.lookup::<Infallible>(child_id).await.unwrap()
    };

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
async fn start_child_no_ack_with_regs() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let s1 = Service::new();
    let s2 = Service::new_with_label("a-service");

    assert!(s1.resolve().is_none());
    assert!(s2.resolve().is_none());

    let sup =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    sup.set_trap_exit(true).await;

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());
    let start_child = start_child::without_ack(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::NoAck,
        [s1.to_owned(), s2.to_owned()],
    );

    assert!(s1.resolve().is_none());
    assert!(s2.resolve().is_none());

    let child = {
        let child_id = start_child.start_child(system.to_owned(), ()).await.unwrap();
        let _ = init_rx.await;
        registry.lookup::<Infallible>(child_id).await.unwrap()
    };

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