use futures::channel::{mpsc, oneshot};
use futures::StreamExt;

#[tokio::test]
async fn test() {
	let t0 = std::time::Instant::now();

	let (mpsc_tx, mut mpsc_rx) = mpsc::unbounded();

	let client_running = async move {
		for i in 0..crate::ITERATIONS {
			let (oneshot_tx, oneshot_rx) = oneshot::channel();

			mpsc_tx
				.unbounded_send((i, [0; crate::PAYLOAD_SIZE], oneshot_tx))
				.expect("mpsc_tx failure");
			assert_eq!(i, oneshot_rx.await.expect("oneshot_rx failure"));
		}
	};

	let server_running = async move {
		while let Some((rq_id, _payload, reply_to)) = mpsc_rx.next().await {
			reply_to.send(rq_id).expect("oneshot_tx failure");
		}
	};

	futures::future::join(server_running, client_running).await;

	eprintln!("ELAPSED: {:?}", t0.elapsed());
}
