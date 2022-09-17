use std::future::Future;
use std::time::Duration;
use tokio::time::Timeout;

pub trait FutureTimeoutExt: Future + Sized {
    fn timeout(self, timeout: Duration) -> Timeout<Self> {
        tokio::time::timeout(timeout, self)
    }
}
impl<T> FutureTimeoutExt for T where T: Future {}
