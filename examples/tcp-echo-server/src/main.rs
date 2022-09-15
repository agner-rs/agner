use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use agner::actors::{ArcError, BoxError, Context, System};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use agner::sup::fixed::{self, ChildSpec};
use agner::sup::{dynamic, Registered};

struct WorkerArgs {
    tcp_stream: TcpStream,
    peer_addr: SocketAddr,
}
async fn worker(context: &mut Context<Infallible>, args: WorkerArgs) -> Result<(), BoxError> {
    context.init_ack(Default::default());

    let WorkerArgs { mut tcp_stream, peer_addr } = args;
    let (tcp_read_half, mut tcp_write_half) = tcp_stream.split();
    let tcp_buf_read = tokio::io::BufReader::new(tcp_read_half);
    let mut tcp_read_lines = tcp_buf_read.lines();

    log::info!("[{}] worker [peer-addr: {}]", context.actor_id(), peer_addr);

    while let Some(line) = tcp_read_lines.next_line().await? {
        tcp_write_half.write_all(line.as_bytes()).await?;
        tcp_write_half.write_all(b"\n").await?;
        tcp_write_half.flush().await?;
    }

    log::info!("[{}] peer-gone [peer-addr: {}]", context.actor_id(), peer_addr);

    Ok(())
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
    context.init_ack(Default::default());

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

    let bind_addr = "127.0.0.1:8090".parse::<SocketAddr>().unwrap();

    let worker_sup = Registered::new();

    let tcp_acceptor_spec = {
        let args = TcpAcceptorArgs { bind_addr, worker_sup: worker_sup.to_owned() };
        fixed::child_spec(tcp_acceptor, fixed::arg_clone(args))
            .with_name("tcp-acceptor")
            .with_init_timeout(Duration::from_secs(3))
    };

    let worker_sup_spec = {
        let make_worker_args = |(tcp_stream, peer_addr)| WorkerArgs { tcp_stream, peer_addr };
        let make_sup_args = move || dynamic::child_spec(worker, make_worker_args);
        fixed::child_spec(dynamic::dynamic_sup, fixed::arg_call(make_sup_args))
            .with_name("worker-sup")
            .with_init_timeout(Duration::from_secs(1))
            .register(worker_sup.to_owned())
    };

    let restart_strategy = ();
    let top_sup_spec = fixed::SupSpec::new(restart_strategy)
        .with_child(tcp_acceptor_spec)
        .with_child(worker_sup_spec);

    let top_sup = system.spawn(fixed::fixed_sup, top_sup_spec, Default::default()).await?;

    Err(system.wait(top_sup).await)
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    run().await.expect("Failure")
}
