use std::convert::Infallible;
use std::net::SocketAddr;

use agner_actors::{ArcError, BoxError, Context, System};
use tokio::net::{TcpListener, TcpStream};

use agner_sup::fixed::{self, ChildSpec};
use agner_sup::{dynamic, Registered};

struct WorkerArgs {
    tcp_stream: TcpStream,
    peer_addr: SocketAddr,
}
async fn worker(_context: &mut Context<Infallible>, _args: WorkerArgs) {
    std::future::pending().await
}

#[derive(Debug, Clone)]
struct TcpAcceptorArgs {
    bind_addr: SocketAddr,
    worker_sup: Registered,
}
async fn tcp_acceptor(
    context: &mut Context<Infallible>,
    args: TcpAcceptorArgs,
) -> Result<(), BoxError> {
    let tcp_listener = TcpListener::bind(args.bind_addr).await?;

    loop {
        let (tcp_stream, peer_addr) = tcp_listener.accept().await?;
        if let Some(worker_sup) = args.worker_sup.get() {
            if let Err(reason) =
                dynamic::start_child(&context.system(), worker_sup, (tcp_stream, peer_addr)).await
            {
                log::warn!("worker-sup invocation error: {}", reason);
            }
        } else {
            log::warn!("worker-sup is not ready");
        }
    }
}

async fn run() -> Result<(), ArcError> {
    log::info!("starting up...");

    let system = System::new(Default::default());

    let bind_addr = "127.0.0.1:8090".parse().unwrap();
    let worker_sup = Registered::new();

    let restart_strategy = ();
    let top_sup = system
        .spawn(
            fixed::fixed_sup,
            fixed::SupSpec::new(restart_strategy)
                .with_child(
                    fixed::child_spec(
                        tcp_acceptor,
                        fixed::arg_clone(TcpAcceptorArgs {
                            bind_addr,
                            worker_sup: worker_sup.to_owned(),
                        }),
                    )
                    .with_name("tcp-acceptor"),
                )
                .with_child(
                    fixed::child_spec(
                        dynamic::dynamic_sup,
                        fixed::arg_call(|| {
                            dynamic::child_spec(worker, |(tcp_stream, peer_addr)| WorkerArgs {
                                tcp_stream,
                                peer_addr,
                            })
                        }),
                    )
                    .with_name("worker-sup")
                    .register(worker_sup.to_owned()),
                ),
            Default::default(),
        )
        .await?;

    Err(system.wait(top_sup).await)
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    run().await.expect("Failure")
}