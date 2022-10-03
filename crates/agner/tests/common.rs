use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use agner_actors::{System, SystemConfig};

#[allow(unused)]
pub const SMALL_SYSTEM_SIZE: usize = 10;

#[allow(unused)]
pub const SMALL_TIMEOUT: Duration = Duration::from_millis(300);
#[allow(unused)]
pub const EXIT_TIMEOUT: Duration = Duration::from_secs(1);

#[allow(unused)]
macro_rules! agner_test {
    ($test_name: ident, $code: expr) => {
        #[cfg(test)]
        mod $test_name {
            fn test() -> impl std::future::Future {
                $code
            }

            #[test]
            fn mt() {
                $crate::common::run(true, test());
            }
            #[test]
            fn st() {
                $crate::common::run(false, test());
            }
        }
    };
}

#[allow(unused)]
pub fn system(max_actors: usize) -> System {
    let exit_handler = Arc::new(agner::actors::exit_handlers::LogExitHandler);

    System::new(SystemConfig { max_actors, exit_handler, ..Default::default() })
}

#[allow(unused)]
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
