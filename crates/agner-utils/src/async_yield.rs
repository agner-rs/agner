use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub async fn async_yield() {
    Yield(false).await
}

struct Yield(bool);

impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let flag = &mut self.as_mut().0;
        if !*flag {
            *flag = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::future_timeout_ext::FutureTimeoutExt;

    #[tokio::test]
    async fn timeout_working_properly() {
        let t0 = Instant::now();
        assert!(tokio::time::sleep(Duration::from_secs(3))
            .timeout(Duration::from_secs(1))
            .await
            .is_err());
        eprintln!("{:?}", t0.elapsed());
    }

    #[tokio::test]
    async fn timeout_does_not_have_a_chance() {
        let t0 = Instant::now();
        assert!(async move {
            for _i in 1..30 {
                std::thread::sleep(Duration::from_millis(100))
            }
        }
        .timeout(Duration::from_secs(1))
        .await
        .is_ok());
        eprintln!("{:?}", t0.elapsed());
    }

    #[tokio::test]
    async fn timeout_works_again() {
        let t0 = Instant::now();
        assert!(async move {
            for _i in 1..30 {
                std::thread::sleep(Duration::from_millis(100));
                crate::async_yield::async_yield().await
            }
        }
        .timeout(Duration::from_secs(1))
        .await
        .is_err());
        eprintln!("{:?}", t0.elapsed());
    }
}
