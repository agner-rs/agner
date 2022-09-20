use agner_actors::{Exit, System};

use crate::api::TestActor;

#[tokio::test]
async fn test_01_exit_and_wait() {
    let system = System::new(Default::default());

    let actor_01_1 =
        TestActor::<usize>::start(system.to_owned(), Default::default()).await.unwrap();
    let actor_01_2 = actor_01_1.to_owned();

    let () = actor_01_1.exit(Exit::shutdown()).await;

    assert!(actor_01_1.wait().await.is_shutdown());
    assert!(actor_01_2.wait().await.is_shutdown());
}
