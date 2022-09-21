use std::convert::Infallible;
use std::time::Duration;

use agner_actors::{Event, Exit, Signal, System};
use agner_test_actor::{TestActor, TestActorRegistry};
use futures::future;

use crate::common::start_child::{self, InitType, StartChild};
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

    let worker = TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
        .await
        .unwrap();

    let (child_args, init_rx) = TestActor::<Infallible>::prepare_args(registry.to_owned());

    let start_child = start_child::new(
        sup.actor_id(),
        agner_test_actor::behaviour::run,
        child_args,
        InitType::WithAck { init_timeout: INIT_TIMEOUT, stop_timeout: INIT_CANCEL_TIMEOUT },
        [],
    );

    let start_child_result = start_child.start_child(system.to_owned(), ());

    let intermediary = async {
        let intermediary_id = init_rx.await.unwrap();
        let intermediary = registry.lookup::<Infallible>(intermediary_id).await.unwrap();
        intermediary.set_link(worker.actor_id(), true);
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