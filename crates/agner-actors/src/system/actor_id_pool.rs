use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;

use crate::actor_id::ActorID;

const MARKER_ACQUIRED: usize = usize::MAX;

#[derive(Debug, Clone)]
pub struct ActorIDPool(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    system_id: usize,
    next_seq_id: AtomicUsize,
    head: AtomicUsize,
    tail: AtomicUsize,
    slots: Box<[AtomicUsize]>,
}

#[derive(Debug)]
pub struct ActorIDLease {
    inner: Arc<Inner>,
    actor_id: ActorID,
}

impl Deref for ActorIDLease {
    type Target = ActorID;
    fn deref(&self) -> &Self::Target {
        &self.actor_id
    }
}

impl Drop for ActorIDLease {
    fn drop(&mut self) {
        self.inner.release_id(self.actor_id.actor());
    }
}

impl ActorIDPool {
    /// Create a new pool.
    pub fn new(system_id: usize, max_actors: usize) -> Self {
        assert!(max_actors + 1 < MARKER_ACQUIRED);

        let inner = Arc::new(Inner {
            system_id,
            next_seq_id: AtomicUsize::new(0),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(max_actors),
            slots: (0..max_actors)
                .map(AtomicUsize::new)
                .chain(std::iter::once(AtomicUsize::new(MARKER_ACQUIRED)))
                .collect(),
        });

        Self(inner)
    }

    /// Acquire an unused [`ActorID`]
    pub fn acquire_id(&self) -> Option<ActorIDLease> {
        let slot_idx = self.0.take_nonempty_slot()?;
        let slot = &self.0.slots[slot_idx];

        let actor_id = loop {
            let actor_id = slot.load(AtomicOrdering::Relaxed);
            if actor_id == MARKER_ACQUIRED {
                continue
            }
            let actor_id = slot
                .compare_exchange(
                    actor_id,
                    MARKER_ACQUIRED,
                    AtomicOrdering::Relaxed,
                    AtomicOrdering::Relaxed,
                )
                .expect("Concurrent modification of non-empty slot");
            break actor_id
        };

        let seq_id = self.0.next_seq_id();

        let in_use = ActorIDLease {
            inner: Arc::clone(&self.0),
            actor_id: ActorID::new(self.0.system_id, actor_id, seq_id),
        };

        Some(in_use)
    }
}

impl Inner {
    fn next_seq_id(&self) -> usize {
        self.next_seq_id.fetch_add(1, AtomicOrdering::Relaxed)
    }

    fn take_nonempty_slot(&self) -> Option<usize> {
        loop {
            if let Ok(ret) = self.take_nonempty_slot_inner() {
                break ret
            }
        }
    }
    fn take_nonempty_slot_inner(&self) -> Result<Option<usize>, ()> {
        let head = self.head.load(AtomicOrdering::SeqCst);
        let tail = self.tail.load(AtomicOrdering::SeqCst);

        let last = self.slots.len() - 1;
        let head_next = if head < last { head + 1 } else { 0 };

        if head == tail {
            Ok(None)
        } else {
            let idx = self
                .head
                .compare_exchange(head, head_next, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
                .map_err(|_| ())?;
            Ok(Some(idx))
        }
    }

    fn take_empty_slot(&self) -> Option<usize> {
        loop {
            if let Ok(ret) = self.take_empty_slot_inner() {
                break ret
            }
        }
    }

    fn take_empty_slot_inner(&self) -> Result<Option<usize>, ()> {
        let head = self.head.load(AtomicOrdering::SeqCst);
        let tail = self.tail.load(AtomicOrdering::SeqCst);

        let last = self.slots.len() - 1;
        let tail_next = if tail < last { tail + 1 } else { 0 };

        if tail_next == head {
            Ok(None)
        } else {
            let idx = self
                .tail
                .compare_exchange(tail, tail_next, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
                .map_err(|_| ())?;
            Ok(Some(idx))
        }
    }

    fn release_id(&self, actor_id: usize) {
        let slot_idx = self
            .take_empty_slot()
            .expect("An attempt to release an id into already full pool.");
        let slot = &self.slots[slot_idx];

        loop {
            let value_before = slot.load(AtomicOrdering::SeqCst);
            if value_before != MARKER_ACQUIRED {
                continue
            }
            slot.compare_exchange(
                value_before,
                actor_id,
                AtomicOrdering::SeqCst,
                AtomicOrdering::SeqCst,
            )
            .expect("Concurrent modification of empty slot");
            break
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;

    use super::*;

    #[test]
    fn acquire_till_out_of_ids_release_and_retry() {
        let pool = ActorIDPool::new(1, 3);

        let id_1 = pool.acquire_id().expect("out of ids");
        eprintln!("acquired: {}", *id_1);
        let id_2 = pool.acquire_id().expect("out of ids");
        eprintln!("acquired: {}", *id_2);
        let id_3 = pool.acquire_id().expect("out of ids");
        eprintln!("acquired: {}", *id_3);
        assert!(pool.acquire_id().is_none());

        std::mem::drop(id_2);

        let id_4 = pool.acquire_id().expect("out of ids");
        eprintln!("acquired: {}", *id_4);

        std::mem::drop(id_3);
        let id_5 = pool.acquire_id().expect("out of ids");
        eprintln!("acquired: {}", *id_5);
    }

    #[tokio::test]
    async fn concurrently_acquired_ids() {
        const ATTEMPTS: usize = 1_000_000;
        const CONCURRENCY: usize = 1024;
        const MAX_ACTORS: usize = 512;

        let pool = ActorIDPool::new(2, MAX_ACTORS);

        let (total_attempts, total_retries, max_retries, max_id) =
            futures::stream::iter(0..ATTEMPTS)
                .map(|attempt_id| {
                    let pool = pool.to_owned();
                    async move {
                        let mut retries = 0;
                        let actor_id = loop {
                            if let Some(id) = pool.acquire_id() {
                                tokio::time::sleep(Duration::ZERO).await;
                                break id
                            } else {
                                retries += 1;
                                tokio::time::sleep(Duration::ZERO).await
                            }
                        };
                        // eprintln!("[{}] {:?} @ {:?}", attempt_id, *actor_id, retries);
                        (attempt_id, retries, *actor_id)
                    }
                })
                .buffer_unordered(CONCURRENCY)
                .fold(
                    (0, 0, 0, ActorID::new(0, 0, 0)),
                    |(total_attempts, total_retries, max_retries, max_id),
                     (_attempt_id, retries, actor_id)| async move {
                        (
                            total_attempts + 1,
                            total_retries + retries,
                            std::cmp::max(max_retries, retries),
                            std::cmp::max(max_id, actor_id),
                        )
                    },
                )
                .await;

        eprintln!("total-attempts: {:?}", total_attempts);
        eprintln!("total-retries:  {:?}", total_retries);
        eprintln!("max-retries:    {:?}", max_retries);
        eprintln!("max-id:         {}", max_id);
    }
}
