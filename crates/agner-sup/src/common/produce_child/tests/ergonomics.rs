use std::convert::Infallible;

use agner_actors::{Context, System};
use agner_test_actor::{TestActor, TestActorRegistry};
use futures::{future, StreamExt};

use crate::common::{args_factory, produce_child, InitType, ProduceChild};
use crate::Service;

#[derive(Debug, Clone)]
struct ConnMgrArgs {
    #[allow(unused)]
    max_conns: usize,
}
async fn conn_mgr(context: &mut Context<Infallible>, args: ConnMgrArgs) {
    log::info!("[{}] conn-mgr with args {:?}", context.actor_id(), args);
    std::future::pending().await
}

#[derive(Debug, Clone)]
struct ConnArgs {
    conn_mgr: Service,
}
#[derive(Debug)]
struct TcpStream(usize);

async fn conn(context: &mut Context<Infallible>, (conn_args, tcp_stream): (ConnArgs, TcpStream)) {
    log::info!(
        "[{}] conn [conn-mgr: {:?}, tcp-stream: {:?}]",
        context.actor_id(),
        conn_args.conn_mgr.resolve().map(|id| id.to_string()),
        tcp_stream
    );
    std::future::pending().await
}

#[tokio::test]
async fn ergonomics() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let registry = TestActorRegistry::new();
    let system = System::new(Default::default());

    let top_sup = TestActor::<Infallible>::start(registry, system.to_owned(), Default::default())
        .await
        .unwrap();

    let conn_mgr_svc = Service::new_with_label("conn-mgr");
    let mut conn_mgr_spec = produce_child::new(
        conn_mgr,
        args_factory::clone(ConnMgrArgs { max_conns: 32 }),
        InitType::NoAck,
        vec![conn_mgr_svc.to_owned()],
    );

    let conn_args = ConnArgs { conn_mgr: conn_mgr_svc };
    let mut conn_spec = produce_child::new(
        conn,
        args_factory::map(move |tcp_stream| (conn_args.to_owned(), tcp_stream)),
        InitType::NoAck,
        vec![],
    );

    let _conn_mgr_id = conn_mgr_spec
        .produce(top_sup.actor_id(), ())
        .start_child(system.to_owned())
        .await
        .unwrap();

    let conn_ids = (0..3)
        .map(TcpStream)
        .map(|tcp_stream| conn_spec.produce(top_sup.actor_id(), tcp_stream))
        .map(|start_child| start_child.start_child(system.to_owned()));
    let _conn_ids = future::join_all(conn_ids)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for actor_id in system.all_actors().collect::<Vec<_>>().await {
        let info = system.actor_info(actor_id).await.unwrap();
        log::info!(
            "[{}] {}({}) <- {}",
            actor_id,
            info.behaviour,
            info.args_type,
            info.message_type
        );
    }
}
