use crate::spsc;

#[derive(Debug)]
pub struct PipeRx<T>(spsc::Receiver<T>);

#[derive(Debug)]
pub struct PipeTx<T>(spsc::Sender<T>);

pub fn new<M>(max_len: usize) -> (PipeTx<M>, PipeRx<M>) {
    let (tx, rx) = spsc::channel(max_len);
    (PipeTx(tx), PipeRx(rx))
}

impl<T> PipeTx<T>
where
    T: Unpin,
{
    pub async fn send(&mut self, message: T) -> Result<(), T> {
        self.0.send(message, false).await
    }
}

impl<T> PipeRx<T>
where
    T: Unpin,
{
    pub async fn recv(&mut self) -> T {
        self.0.recv(true).await.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_test() {
        const LEN: usize = 30;
        let (mut inbox_w, mut inbox_r) = new::<usize>(LEN);

        for i in 1..1_000_000 {
            if let Err(rejected) = inbox_w.send(i).await {
                assert_eq!(rejected, i);

                let popped = inbox_r.recv().await;
                assert_eq!(popped + LEN, rejected);
                assert!(inbox_w.send(rejected).await.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn futures_mpsc_test() {
        use futures::StreamExt;

        const LEN: usize = 30;
        let (mut inbox_w, mut inbox_r) = futures::channel::mpsc::channel::<usize>(LEN - 1);

        for i in 1..1_000_000 {
            if let Err(rejected) = inbox_w.try_send(i) {
                let rejected = rejected.into_inner();
                assert_eq!(rejected, i);

                let popped = inbox_r.next().await.expect("channel closed?");
                assert_eq!(popped + LEN, rejected);
                assert!(inbox_w.try_send(rejected).is_ok());
            }
        }
    }

    #[tokio::test]
    async fn tokio_mpsc_test() {
        use tokio::sync::mpsc::error::TrySendError;

        const LEN: usize = 30;
        let (inbox_w, mut inbox_r) = tokio::sync::mpsc::channel::<usize>(LEN);

        for i in 1..1_000_000 {
            if let Err(rejected) = inbox_w.try_send(i) {
                let rejected = match rejected {
                    TrySendError::Full(rejected) => rejected,
                    _ => panic!("unexpected channel error"),
                };
                assert_eq!(rejected, i);

                let popped = inbox_r.recv().await.expect("channel closed?");
                assert_eq!(popped + LEN, rejected);
                assert!(inbox_w.try_send(rejected).is_ok());
            }
        }
    }
}
