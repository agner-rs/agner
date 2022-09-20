use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures::lock::BiLock;

#[cfg(test)]
mod tests;

pub fn channel<T>(max_len: usize) -> (Sender<T>, Receiver<T>) {
    let inner =
        Inner { queue: Default::default(), max_len, sender_waker: None, receiver_waker: None };

    let (sender, receiver) = BiLock::new(inner);
    (Sender(sender), Receiver(receiver))
}

#[derive(Debug)]
pub struct Receiver<T>(BiLock<Inner<T>>);

#[derive(Debug)]
pub struct Sender<T>(BiLock<Inner<T>>);

impl<T> Receiver<T>
where
    T: Unpin,
{
    pub fn recv<'a>(&'a mut self, should_block: bool) -> impl Future<Output = Option<T>> + 'a {
        Receive { lock: &self.0, should_block }
    }

    pub async fn len(&self) -> (usize, usize) {
        let locked = self.0.lock().await;
        (locked.queue.len(), locked.max_len)
    }
}

impl<T> Sender<T>
where
    T: Unpin,
{
    pub fn send(
        &mut self,
        item: T,
        should_block: bool,
    ) -> impl Future<Output = Result<(), T>> + '_ {
        Send { lock: &self.0, should_block, item: Some(item) }
    }

    pub async fn len(&self) -> (usize, usize) {
        let locked = self.0.lock().await;
        (locked.queue.len(), locked.max_len)
    }
}

#[derive(Debug)]
struct Inner<T> {
    queue: VecDeque<T>,
    max_len: usize,
    sender_waker: Option<Waker>,
    receiver_waker: Option<Waker>,
}

#[pin_project::pin_project]
struct Receive<'a, T> {
    lock: &'a BiLock<Inner<T>>,
    should_block: bool,
}

#[pin_project::pin_project]
struct Send<'a, T> {
    lock: &'a BiLock<Inner<T>>,
    should_block: bool,
    item: Option<T>,
}

impl<'a, T> Future for Receive<'a, T>
where
    T: Unpin,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut locked = futures::ready!(this.lock.poll_lock(cx));
        let _ = locked.receiver_waker.take();

        match (locked.queue.pop_front(), this.should_block) {
            (Some(item), _) => {
                if let Some(waker) = locked.sender_waker.take() {
                    waker.wake();
                }
                Poll::Ready(Some(item))
            },
            (None, false) => Poll::Ready(None),
            (None, true) => {
                let should_be_none = locked.receiver_waker.replace(cx.waker().to_owned());
                assert!(should_be_none.is_none());
                Poll::Pending
            },
        }
    }
}

impl<'a, T> Future for Send<'a, T>
where
    T: Unpin,
{
    type Output = Result<(), T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut locked = futures::ready!(this.lock.poll_lock(cx));
        let _ = locked.sender_waker.take();

        match (locked.queue.len() < locked.max_len, this.should_block) {
            (true, _) => {
                let item = this.item.take().expect("Item empty");
                locked.queue.push_back(item);
                if let Some(waker) = locked.receiver_waker.take() {
                    waker.wake();
                }
                Poll::Ready(Ok(()))
            },
            (false, false) => {
                let item = this.item.take().expect("Item empty");
                Poll::Ready(Err(item))
            },
            (false, true) => {
                let should_be_none = locked.sender_waker.replace(cx.waker().to_owned());
                assert!(should_be_none.is_none());
                Poll::Pending
            },
        }
    }
}
