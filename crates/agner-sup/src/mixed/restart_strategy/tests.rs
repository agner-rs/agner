use std::sync::atomic::{AtomicUsize, Ordering};

use agner_actors::{ActorID, Exit};

use crate::mixed::restart_intensity::*;
use crate::mixed::restart_strategy::common_decider::*;
use crate::mixed::restart_strategy::{Action, ChildType, Decider};

mod basic;

fn next_id() -> ActorID {
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    let id = NEXT.fetch_add(1, Ordering::Relaxed);

    format!("42.{}.{}", id, id).parse().unwrap()
}

fn next_tick() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

type ID = &'static str;
type TestDecider = CommonDecider<ID, usize, usize>;
