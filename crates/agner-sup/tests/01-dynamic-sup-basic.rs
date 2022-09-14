use agner_actors::{Context, ExitReason, System};

mod common;

use agner_sup::dynamic;

#[test]
fn dynamic_sup_basic_test() {
    #[allow(unused)]
    struct WorkerArgs {
        group_name: &'static str,
        worker_id: usize,
    }
    enum WorkerMessage {}
    async fn worker_behaviour(_context: &mut Context<WorkerMessage>, _arg: WorkerArgs) {
        std::future::pending().await
    }

    common::run(async {
        let system = System::new(Default::default());
        let sup = system
            .spawn(
                dynamic::dynamic_sup,
                dynamic::child_spec(worker_behaviour, {
                    let mut id: usize = 0;
                    move |()| {
                        id += 1;
                        WorkerArgs { group_name: "group-name", worker_id: id }
                    }
                }),
                Default::default(),
            )
            .await
            .unwrap();

        for _ in 0..100 {
            let child_id =
                dynamic::start_child(&system, sup, ()).await.expect("Failed to start a child");
            log::info!("child-started: {}", child_id);
        }

        system.exit(sup, ExitReason::Shutdown(None)).await;
        system.wait(sup).await;
    })
}
