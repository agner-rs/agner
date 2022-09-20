use std::future::Future;

use agner_actors::{System, SystemConfig};

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
    System::new(SystemConfig { max_actors, ..Default::default() })
}

#[allow(unused)]
pub fn run<F>(multi_thread: bool, f: F) -> F::Output
where
    F: Future,
{
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
