use agner::test_actor::{TestActor, TestActorRegistry};
use agner::utils::future_timeout_ext::FutureTimeoutExt;
use agner::utils::std_error_pp::StdErrorPP;
use std::convert::Infallible;
use std::time::Duration;

const EXIT_TIMEOUT: Duration = Duration::from_secs(1);

#[macro_use]
mod common;

agner_test!(actor_invokes_context_exit_and_then_exits, async move {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = crate::common::system(32);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();
    a1.exit(Default::default()).await;
    let reason = a1.wait().timeout(EXIT_TIMEOUT).await.unwrap();

    assert!(reason.is_normal(), "{}", reason.pp());
});

agner_test!(actor_is_terminated_via_system_exit, async move {
    use crate::*;

    let registry = TestActorRegistry::new();
    let system = crate::common::system(32);

    let a1 =
        TestActor::<Infallible>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    system.exit(a1.actor_id(), Default::default()).await;
    let reason = a1.wait().timeout(EXIT_TIMEOUT).await.unwrap();
    assert!(reason.is_normal(), "{}", reason.pp());
});

agner_test!(actor_with_trapexit_is_terminated_via_system_exit_receives_signal, async move {
    // use crate::*;
});
