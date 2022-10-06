#![cfg(feature = "reg")]

use std::convert::Infallible;

use agner_actors::{Context, System};
use agner_reg::Service;
use agner_test_actor::{TestActor, TestActorRegistry};

use futures::{future, StreamExt};

use crate::common::{args_factory, produce_child, InitType, ProduceChild, WithRegisteredService};

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

    let conn_mgr_svc = Service::new();
    let mut conn_mgr_spec = produce_child::new(
        conn_mgr,
        args_factory::clone(ConnMgrArgs { max_conns: 32 }),
        InitType::NoAck,
    )
    .with_registered_service(conn_mgr_svc.to_owned());

    let conn_args = ConnArgs { conn_mgr: conn_mgr_svc };
    let mut conn_spec = produce_child::new(
        conn,
        args_factory::map(move |tcp_stream| (conn_args.to_owned(), tcp_stream)),
        InitType::NoAck,
    );

    let _conn_mgr_id =
        conn_mgr_spec.produce(system.to_owned(), top_sup.actor_id(), ()).await.unwrap();

    let conn_ids = (0..3)
        .map(TcpStream)
        .map(|tcp_stream| conn_spec.produce(system.to_owned(), top_sup.actor_id(), tcp_stream));
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
