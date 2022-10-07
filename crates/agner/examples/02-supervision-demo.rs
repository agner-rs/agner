use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agner::actors::Exit;
use agner::reg::Service;
use agner::sup::common::WithAck;
use agner::sup::mixed::{self, AllForOne, RestartIntensity};
use agner::sup::uniform;
use agner_sup::mixed::MixedChildSpec;
use agner_sup::uniform::UniformChildSpec;
use tokio::net::UnixStream;
use tokio::signal::unix::SignalKind;

mod common;

fn main() {
    let multi_thread = std::env::var("MULTI_THREAD").ok().filter(|s| s == "1").is_some();
    let max_actors: usize =
        std::env::var("MAX_ACTORS").ok().unwrap_or("1024".to_owned()).parse().unwrap();
    let uds_acceptors_count: usize = std::env::var("UDS_ACCEPTORS_COUNT")
        .ok()
        .unwrap_or("16".to_owned())
        .parse()
        .unwrap();
    let bind_uds = std::env::var("BIND_UDS")
        .map(PathBuf::from)
        .map(Arc::<Path>::from)
        .expect("BIND_UDS is a required env-var");

    common::run(multi_thread, async {
        let system = common::system(max_actors);

        let restart_intensity = RestartIntensity::new(0, Duration::ZERO);
        let restart_strategy = AllForOne::new(restart_intensity);

        let fanout_svc = Service::new();
        let uds_conn_sup_svc = Service::new();

        let top_sup_spec = mixed::SupSpec::new(restart_strategy)
            .with_child(
                MixedChildSpec::mixed("fanout")
                    .behaviour(actors::fanout::run)
                    .args_clone(())
                    .register(fanout_svc.to_owned()),
            )
            .with_child(
                MixedChildSpec::mixed("uds-conn-sup")
                    .behaviour(uniform::run)
                    .args_clone(uniform::SupSpec::new(
                        UniformChildSpec::uniform()
                            .behaviour(actors::connection::run::<UnixStream>)
                            .args_call1(move |uds_stream| (fanout_svc.to_owned(), uds_stream)),
                    ))
                    .init_type(WithAck::new())
                    .register(uds_conn_sup_svc.to_owned()),
            )
            .with_child(
                MixedChildSpec::mixed("uds-interface")
                    .behaviour(actors::uds_interface::run)
                    .args_clone((bind_uds, uds_acceptors_count, uds_conn_sup_svc.to_owned())),
            );

        let top_sup = system
            .spawn(mixed::run, top_sup_spec, Default::default())
            .await
            .expect("Failed to spawn top-sup");

        let top_sup_waiting = system.wait(top_sup);
        let mut sig_int = tokio::signal::unix::signal(SignalKind::interrupt()).unwrap();
        let mut sig_term = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();

        if let Some(exit_reason) = tokio::select! {
            _top_sup_exited = top_sup_waiting => None,
            _sig_int = sig_int.recv() => Some(Exit::shutdown()),
            _sig_term = sig_term.recv() => Some(Exit::shutdown()),
        } {
            system.exit(top_sup, exit_reason).await;
            system.wait(top_sup).await;
        }
    });
}

mod actors {
    pub mod fanout {
        use std::collections::HashSet;
        use std::convert::Infallible;
        use std::sync::Arc;

        use agner::actors::{ActorID, Context, Never};
        use agner::init_ack::ContextInitAckExt;
        use tokio::sync::oneshot;

        use crate::actors::connection;

        pub enum Message {
            Register(ActorID),
            Unregister(ActorID),
            Publish(String, oneshot::Sender<Infallible>),
        }

        pub async fn run(context: &mut Context<Message>, _args: ()) -> Never {
            context.init_ack_ok(Default::default());
            log::info!("[{}] Fanout started", context.actor_id());

            let mut connections = HashSet::new();
            loop {
                match context.next_message().await {
                    Message::Register(connection) =>
                        if connections.insert(connection) {
                            let wait = context.system().wait(connection);
                            context
                                .future_to_inbox(async move {
                                    let _exit_reason = wait.await;
                                    Message::Unregister(connection)
                                })
                                .await;
                        },
                    Message::Unregister(connection) => {
                        connections.remove(&connection);
                    },
                    Message::Publish(mut line, _reply_to) => {
                        line.push('\n');
                        let line = Arc::<str>::from(line);

                        let system = context.system();
                        for connection in connections.iter().copied() {
                            system
                                .send(connection, connection::Message::Publish(line.to_owned()))
                                .await;
                        }
                    },
                }
            }
        }
    }

    pub mod connection {
        use std::sync::Arc;

        use crate::actors::fanout;
        use agner::actors::{ActorID, Context, Exit, Shutdown, SystemWeakRef};
        use agner::init_ack::ContextInitAckExt;
        use agner::reg::Service;
        use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};
        use tokio::sync::oneshot;

        #[derive(Debug)]
        pub enum Message {
            Publish(Arc<str>),
            ReaderLine(String),
            ReaderDone(Option<Exit>),
        }

        pub async fn run<IO>(
            context: &mut Context<Message>,
            (fanout, io): (Service, IO),
        ) -> Result<Shutdown, Exit>
        where
            IO: AsyncRead + AsyncWrite + Send + Sync + 'static,
        {
            let (io_read_half, mut io_write_half) = tokio::io::split(io);
            context
                .system()
                .send(
                    fanout.resolve().ok_or_else(|| Exit::from_message("fanout gone"))?,
                    fanout::Message::Register(context.actor_id()),
                )
                .await;
            context.init_ack_ok(Default::default());

            log::info!("[{}] Connection started", context.actor_id());

            let reader_routine_running = {
                let connection = context.actor_id();
                let system_weak_ref = context.system().rc_downgrade();
                async move {
                    let err_opt =
                        reader_routine(system_weak_ref, connection, io_read_half).await.err();
                    Message::ReaderDone(err_opt)
                }
            };
            context.future_to_inbox(reader_routine_running).await;

            loop {
                match context.next_message().await {
                    Message::ReaderDone(None) => break Ok(Shutdown::new()),
                    Message::ReaderDone(Some(reader_failure)) => break Err(reader_failure),
                    Message::ReaderLine(line) => {
                        let fanout =
                            fanout.resolve().ok_or_else(|| Exit::from_message("fanout gone"))?;
                        let (tx, done) = oneshot::channel();
                        context.system().send(fanout, fanout::Message::Publish(line, tx)).await;
                        let _ = done.await;
                    },
                    Message::Publish(line) => {
                        io_write_half.write_all(line.as_bytes()).await.map_err(Exit::custom)?;
                        io_write_half.flush().await.map_err(Exit::custom)?;
                    },
                }
            }
        }

        async fn reader_routine<IO>(
            system: SystemWeakRef,
            connection: ActorID,
            io: IO,
        ) -> Result<(), Exit>
        where
            IO: AsyncRead + Unpin,
        {
            let io_buf_read = tokio::io::BufReader::new(io);
            let mut io_read_lines = io_buf_read.lines();
            while let Some(line) = io_read_lines.next_line().await.map_err(Exit::custom)? {
                let system =
                    system.rc_upgrade().ok_or_else(|| Exit::from_message("system gone"))?;
                system.send(connection, Message::ReaderLine(line)).await;
            }

            Ok(())
        }
    }

    pub mod uds_acceptor {
        use std::sync::Arc;

        use agner::actors::{Context, Exit, Never};
        use agner::init_ack::ContextInitAckExt;
        use agner::reg::Service;
        use agner::sup::uniform;
        use tokio::net::UnixListener;

        pub mod api {}

        #[derive(Debug)]
        pub enum Message {}

        pub async fn run(
            context: &mut Context<Message>,
            (uds_listener, conn_sup): (Arc<UnixListener>, Service),
        ) -> Result<Never, Exit> {
            context.init_ack_ok(Default::default());

            log::info!("[{}] UDS-acceptor started", context.actor_id());

            loop {
                let (uds_stream, _) = uds_listener.accept().await.map_err(Exit::custom)?;
                let conn_sup = conn_sup
                    .resolve()
                    .ok_or_else(|| Exit::from_message("Failed to resolve conn_sup"))?;
                uniform::start_child(&context.system(), conn_sup, uds_stream)
                    .await
                    .map_err(Exit::custom)?;
            }
        }
    }

    pub mod uds_interface {
        use std::path::Path;
        use std::sync::Arc;
        use std::time::Duration;

        use agner::actors::{Context, Exit, SpawnOpts};
        use agner::init_ack::InitAckTx;
        use agner::reg::Service;
        use agner::sup::mixed;
        use agner::sup::mixed::{OneForOne, RestartIntensity};
        use agner_sup::common::WithAck;
        use agner_sup::mixed::MixedChildSpec;
        use tokio::net::UnixListener;

        #[derive(Debug)]
        pub enum Message {}

        pub async fn run(
            context: &mut Context<Message>,
            (bind_path, acceptors_count, conn_sup): (Arc<Path>, usize, Service),
        ) -> Result<(), Exit> {
            let uds_listener = UnixListener::bind(bind_path.as_ref()).map_err(Exit::custom)?;
            let unlink_on_drop = UnlinkOnDrop(bind_path.to_owned());
            let uds_listener = Arc::new(uds_listener);

            let mut sup_spawn_opts = SpawnOpts::default().with_data(unlink_on_drop);
            if let Some(init_ack_tx) = context.take::<InitAckTx>() {
                log::info!("init-ack-tx: {:?}", init_ack_tx);
                sup_spawn_opts = sup_spawn_opts.with_data(init_ack_tx);
            }

            let restart_intensity = RestartIntensity::new(5, Duration::from_secs(60));
            let restart_strategy = OneForOne::new(restart_intensity);
            let mut sup_spec = mixed::SupSpec::new(restart_strategy);

            for acceptor_id in 0..acceptors_count {
                let child_spec = MixedChildSpec::mixed(acceptor_id)
                    .behaviour(crate::actors::uds_acceptor::run)
                    .args_clone((uds_listener.to_owned(), conn_sup.to_owned()))
                    .init_type(WithAck::new());

                sup_spec = sup_spec.with_child(child_spec);
            }

            let acceptors_sup = context
                .system()
                .spawn(mixed::run, sup_spec, sup_spawn_opts)
                .await
                .map_err(Exit::custom)?;

            log::info!(
                "[{}] interface [bind-path: {:?}; acceptors_sup: {}]",
                context.actor_id(),
                bind_path,
                acceptors_sup
            );

            Ok(())
        }

        #[derive(Debug)]
        struct UnlinkOnDrop(Arc<Path>);

        impl Drop for UnlinkOnDrop {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(self.0.as_ref());
            }
        }
    }
}
