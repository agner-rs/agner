use agner_actors::{ArcError, System};

mod room {
	use agner_actors::{ActorID, BoxError, Context, Event};
	use std::collections::HashMap;
	use std::net::SocketAddr;
	use std::sync::Arc;

	use super::conn;

	pub enum Message {
		Join(ActorID, SocketAddr),
		Post(ActorID, Arc<str>),
	}

	pub async fn run(context: &mut Context<Message>, arg: ()) -> Result<(), BoxError> {
		let mut participants = HashMap::new();

		loop {
			match context.next_event().await {
				Event::Signal { .. } => unreachable!(),
				Event::Message(Message::Join(actor_id, peer_addr)) => {
					for participant_actor_id in participants.keys().copied() {
						context
							.system()
							.send(participant_actor_id, conn::Message::Joined(peer_addr))
							.await;
					}

					participants.insert(actor_id, peer_addr);
				},
				Event::Message(Message::Post(actor_id, message)) => {
					if let Some(from_addr) = participants.get(&actor_id).copied() {
						for participand_actor_id in
							participants.keys().copied().filter(|p| *p != actor_id)
						{
							context
								.system()
								.send(
									participand_actor_id,
									conn::Message::Posted(from_addr, Arc::clone(&message)),
								)
								.await;
						}
					}
				},
			}
		}
	}
}

mod conn {
	use agner_actors::{ActorID, BoxError, Context, Event};
	use std::net::SocketAddr;
	use std::sync::Arc;
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
	use tokio::net::TcpStream;

	use super::room;

	pub struct Args {
		pub tcp_stream: TcpStream,
		pub peer_addr: SocketAddr,
		pub room: ActorID,
	}

	pub enum Message {
		Joined(SocketAddr),
		Left(SocketAddr),
		Posted(SocketAddr, Arc<str>),
	}

	pub async fn run(context: &mut Context<Message>, mut args: Args) -> Result<(), BoxError> {
		context
			.system()
			.send(args.room, room::Message::Join(context.actor_id(), args.peer_addr))
			.await;

		let (read_half, mut write_half) = args.tcp_stream.split();
		let mut read_lines = BufReader::new(read_half).lines();

		loop {
			tokio::select! {
				next_line = read_lines.next_line() => {
					let next_line = next_line?;
					let next_line = next_line.ok_or("EOF")?;

					context.system().send(args.room, room::Message::Post(context.actor_id(), next_line.into())).await;
				},
				event = context.next_event() => {
					match event {
						Event::Message(Message::Joined(addr)) => {
							let message = format!("JOINED [{}]\n", addr);
							write_half.write_all(message.as_bytes()).await?;
							write_half.flush().await?;
						}
						Event::Message(Message::Left(addr)) => {
							let message = format!("LEFT [{}]\n", addr);
							write_half.write_all(message.as_bytes()).await?;
							write_half.flush().await?;
						}
						Event::Message(Message::Posted(from, message)) => {
							let message = format!("[{}] {}\n", from, message);
							write_half.write_all(message.as_bytes()).await?;
							write_half.flush().await?;
						},
						Event::Signal {..} => unreachable!()
					}
				}
			}
		}
	}
}

mod acceptor {
	use std::net::SocketAddr;

	use agner_actors::{ActorID, BoxError, Context};
	use agner_sup::dynamic;
	use tokio::net::TcpListener;

	pub struct Args {
		pub conn_sup: ActorID,
		pub bind_addr: SocketAddr,
	}

	pub type Message = std::convert::Infallible;

	pub async fn run(context: &mut Context<Message>, args: Args) -> Result<(), BoxError> {
		let tcp_listener = TcpListener::bind(args.bind_addr).await?;

		loop {
			let (tcp_stream, peer_addr) = tcp_listener.accept().await?;
			dynamic::start_child(&context.system(), args.conn_sup, (tcp_stream, peer_addr)).await?;
		}
	}
}

#[tokio::main]
async fn main() {
	let _ = dotenv::dotenv();
	let _ = pretty_env_logger::try_init_timed();

	run().await.expect("Failure")
}

async fn run() -> Result<(), ArcError> {
	let system = System::new(Default::default());

	let room = system.spawn(room::run, (), Default::default()).await?;
	let conn_sup =
		system
			.spawn(
				agner_sup::dynamic::dynamic_sup,
				agner_sup::dynamic::child_spec(conn::run, move |(tcp_stream, peer_addr)| {
					conn::Args { tcp_stream, peer_addr, room }
				}),
				Default::default(),
			)
			.await?;
	let acceptor = system
		.spawn(
			acceptor::run,
			acceptor::Args { bind_addr: "127.0.0.1:8090".parse().unwrap(), conn_sup },
			Default::default(),
		)
		.await?;

	let failure = ArcError::clone(&tokio::select! {
		room_down = system.wait(room) => room_down,
		conn_sup_down = system.wait(conn_sup) => conn_sup_down,
		acceptor_down = system.wait(acceptor) => acceptor_down,
	});

	Err(failure)
}
