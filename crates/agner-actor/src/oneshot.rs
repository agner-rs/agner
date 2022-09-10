use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::oneshot;

pub fn channel<T>() -> (Tx<T>, Rx<T>) {
	let (tx, rx) = oneshot::channel();

	(Tx(tx), Rx(rx))
}

#[derive(Debug)]
pub struct Tx<T>(oneshot::Sender<T>);

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Rx<T>(#[pin] oneshot::Receiver<T>);

impl<T> Tx<T> {
	pub fn send(self, value: T) {
		let _ = self.0.send(value);
	}
}

impl<T> Future for Rx<T>
where
	oneshot::Receiver<T>: Future<Output = Result<T, oneshot::error::RecvError>>,
{
	type Output = Option<T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project().0.poll(cx).map(Result::ok)
	}
}
