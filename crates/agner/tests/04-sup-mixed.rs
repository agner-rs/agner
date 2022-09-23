#![cfg(feature = "test-actor")]
#![cfg(feature = "sup")]

use std::convert::Infallible;
use std::time::Duration;

use agner::actors::Exit;
use agner::sup::common::{args_factory, produce_child, InitType};
use agner::sup::mixed::{ChildSpec, OneForOne, RestartIntensity, SupSpec};
use agner::sup::Service;
use agner::test_actor::{TestActor, TestActorRegistry};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use agner_sup::mixed::{AllForOne, RestForOne};

#[macro_use]
mod common;

const SHORT_PAUSE: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error)]
#[error("a failure")]
struct Failure;

agner_test!(sup_one_for_one_basic, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let child_1_svc = Service::new();
    let child_1_spec = ChildSpec::new(
        "one",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_1_svc.to_owned()],
        ),
    );

    let child_2_svc = Service::new();
    let child_2_spec = ChildSpec::new(
        "two",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_2_svc.to_owned()],
        ),
    );

    let child_3_svc = Service::new();
    let child_3_spec = ChildSpec::new(
        "three",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_3_svc.to_owned()],
        ),
    );

    let restart_intensity = RestartIntensity::new(3, Duration::from_secs(60));
    let restart_strategy = OneForOne::new(restart_intensity);
    let sup_spec = SupSpec::new(restart_strategy)
        .with_child(child_1_spec)
        .with_child(child_2_spec)
        .with_child(child_3_spec);

    let sup_pid = system
        .spawn(agner::sup::mixed::supervisor::run, sup_spec, Default::default())
        .await
        .unwrap();

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let child_1_pid = child_1_svc.resolve().unwrap();
    let mut child_2_pid = child_2_svc.resolve().unwrap();
    let child_3_pid = child_3_svc.resolve().unwrap();

    for _ in 0..3 {
        assert!(system.actor_info(child_1_pid).await.is_some());
        assert!(system.actor_info(child_2_pid).await.is_some());
        assert!(system.actor_info(child_3_pid).await.is_some());

        let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
        child_2_api.exit(Exit::custom(Failure)).await;

        ::tokio::time::sleep(SHORT_PAUSE).await;
        assert!(system.actor_info(child_1_pid).await.is_some());
        assert!(system.actor_info(child_2_pid).await.is_none());
        assert!(system.actor_info(child_3_pid).await.is_some());

        child_2_pid = child_2_svc.resolve().unwrap();
    }

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
    child_2_api.exit(Exit::custom(Failure)).await;

    assert!(system
        .wait(sup_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());

    assert!(system
        .wait(child_1_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
    assert!(system
        .wait(child_2_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_custom());
    assert!(system
        .wait(child_3_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
});

agner_test!(sup_all_for_one_basic, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let child_1_svc = Service::new();
    let child_1_spec = ChildSpec::new(
        "one",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_1_svc.to_owned()],
        ),
    );

    let child_2_svc = Service::new();
    let child_2_spec = ChildSpec::new(
        "two",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_2_svc.to_owned()],
        ),
    );

    let child_3_svc = Service::new();
    let child_3_spec = ChildSpec::new(
        "three",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_3_svc.to_owned()],
        ),
    );

    let restart_intensity = RestartIntensity::new(3, Duration::from_secs(60));
    let restart_strategy = AllForOne::new(restart_intensity);
    let sup_spec = SupSpec::new(restart_strategy)
        .with_child(child_1_spec)
        .with_child(child_2_spec)
        .with_child(child_3_spec);

    let sup_pid = system
        .spawn(agner::sup::mixed::supervisor::run, sup_spec, Default::default())
        .await
        .unwrap();

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let mut child_1_pid = child_1_svc.resolve().unwrap();
    let mut child_2_pid = child_2_svc.resolve().unwrap();
    let mut child_3_pid = child_3_svc.resolve().unwrap();

    for _ in 0..3 {
        assert!(system.actor_info(child_1_pid).await.is_some());
        assert!(system.actor_info(child_2_pid).await.is_some());
        assert!(system.actor_info(child_3_pid).await.is_some());

        let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
        child_2_api.exit(Exit::custom(Failure)).await;

        ::tokio::time::sleep(SHORT_PAUSE).await;
        assert!(system.actor_info(child_1_pid).await.is_none());
        assert!(system.actor_info(child_2_pid).await.is_none());
        assert!(system.actor_info(child_3_pid).await.is_none());

        child_1_pid = child_1_svc.resolve().unwrap();
        child_2_pid = child_2_svc.resolve().unwrap();
        child_3_pid = child_3_svc.resolve().unwrap();
    }

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
    child_2_api.exit(Exit::custom(Failure)).await;

    assert!(system
        .wait(sup_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());

    assert!(system
        .wait(child_1_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
    assert!(system
        .wait(child_2_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_custom());
    assert!(system
        .wait(child_3_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
});

agner_test!(sup_rest_for_one_basic, async {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = common::system(common::SMALL_SYSTEM_SIZE);

    let child_1_svc = Service::new();
    let child_1_spec = ChildSpec::new(
        "one",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_1_svc.to_owned()],
        ),
    );

    let child_2_svc = Service::new();
    let child_2_spec = ChildSpec::new(
        "two",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_2_svc.to_owned()],
        ),
    );

    let child_3_svc = Service::new();
    let child_3_spec = ChildSpec::new(
        "three",
        produce_child::new(
            agner::test_actor::behaviour::run,
            args_factory::call({
                let registry = registry.to_owned();
                move || {
                    let (args, _) = TestActor::<Infallible>::prepare_args(registry.to_owned());
                    args
                }
            }),
            InitType::NoAck,
            vec![child_3_svc.to_owned()],
        ),
    );

    let restart_intensity = RestartIntensity::new(3, Duration::from_secs(60));
    let restart_strategy = RestForOne::new(restart_intensity);
    let sup_spec = SupSpec::new(restart_strategy)
        .with_child(child_1_spec)
        .with_child(child_2_spec)
        .with_child(child_3_spec);

    let sup_pid = system
        .spawn(agner::sup::mixed::supervisor::run, sup_spec, Default::default())
        .await
        .unwrap();

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let child_1_pid = child_1_svc.resolve().unwrap();
    let mut child_2_pid = child_2_svc.resolve().unwrap();
    let mut child_3_pid = child_3_svc.resolve().unwrap();

    for _ in 0..3 {
        assert!(system.actor_info(child_1_pid).await.is_some());
        assert!(system.actor_info(child_2_pid).await.is_some());
        assert!(system.actor_info(child_3_pid).await.is_some());

        let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
        child_2_api.exit(Exit::custom(Failure)).await;

        ::tokio::time::sleep(SHORT_PAUSE).await;
        assert!(system.actor_info(child_1_pid).await.is_some());
        assert!(system.actor_info(child_2_pid).await.is_none());
        assert!(system.actor_info(child_3_pid).await.is_none());

        child_2_pid = child_2_svc.resolve().unwrap();
        child_3_pid = child_3_svc.resolve().unwrap();
    }

    ::tokio::time::sleep(SHORT_PAUSE).await;
    let child_2_api = registry.lookup::<Infallible>(child_2_pid).await.unwrap();
    child_2_api.exit(Exit::custom(Failure)).await;

    assert!(system
        .wait(sup_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());

    assert!(system
        .wait(child_1_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
    assert!(system
        .wait(child_2_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_custom());
    assert!(system
        .wait(child_3_pid)
        .timeout(SHORT_PAUSE)
        .await
        .expect("sup did not terminate")
        .is_shutdown());
});
