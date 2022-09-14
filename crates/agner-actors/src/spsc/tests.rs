use super::*;

use futures::future;

#[tokio::test]
async fn no_blocking_basic() {
	let (mut tx, mut rx) = channel::<usize>(3);
	assert!(tx.send(1, false).await.is_ok());
	assert!(tx.send(2, false).await.is_ok());
	assert!(tx.send(3, false).await.is_ok());
	assert_eq!(tx.send(4, false).await, Err(4));

	assert_eq!(rx.recv(false).await, Some(1));
	assert!(tx.send(4, false).await.is_ok());
	assert_eq!(tx.send(5, false).await, Err(5));

	assert_eq!(rx.recv(false).await, Some(2));
	assert_eq!(rx.recv(false).await, Some(3));
	assert_eq!(rx.recv(false).await, Some(4));
	assert_eq!(rx.recv(false).await, None);
}

#[tokio::test]
async fn blocking_basic() {
	let (mut tx, mut rx) = channel::<usize>(3);
	let producer = async move {
		for i in 1..10 {
			assert!(tx.send(i, true).await.is_ok());
		}
	};
	let consumer = async move {
		for i in 1..10 {
			assert_eq!(rx.recv(true).await, Some(i));
		}
		assert_eq!(rx.recv(false).await, None);
	};

	assert_eq!(future::join(producer, consumer).await, ((), ()));
}
