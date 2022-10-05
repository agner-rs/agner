#![allow(unused)]

use std::future::Future;
use std::sync::Arc;

use agner_actors::{System, SystemConfig};

pub fn system(max_actors: usize) -> System {
    let exit_handler = Arc::new(agner::actors::exit_handlers::LogExitHandler);

    System::new(SystemConfig { max_actors, exit_handler, ..Default::default() })
}

pub fn run<F>(multi_thread: bool, f: F) -> F::Output
where
    F: Future,
{
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    if multi_thread {
        tokio::runtime::Builder::new_multi_thread()
    } else {
        tokio::runtime::Builder::new_current_thread()
    }
    .enable_all()
    .build()
    .expect("Failed to create runtime")
    .block_on(f)
}

fn main() {}
