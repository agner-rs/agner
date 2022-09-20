
use agner_actors::{Exit, System};

use crate::api::TestActor;
use crate::TestActorRegistry;

#[tokio::test]
async fn test_01_exit_and_wait() {
    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let actor_01_id =
        TestActor::<usize>::start(registry.to_owned(), system.to_owned(), Default::default())
            .await
            .unwrap();

    let actor_01_1 = registry.lookup::<usize>(actor_01_id).await.unwrap();
    let actor_01_2 = registry.lookup::<usize>(actor_01_id).await.unwrap();

    let () = actor_01_1.exit(Exit::shutdown()).await;

    assert!(actor_01_1.wait().await.is_shutdown());
    assert!(actor_01_2.wait().await.is_shutdown());
}
