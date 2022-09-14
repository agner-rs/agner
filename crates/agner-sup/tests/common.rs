use std::future::Future;

pub fn run<Fut>(f: Fut) -> Fut::Output
where
    Fut: Future,
{
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    let runtime = tokio::runtime::Builder::
		// new_current_thread()
		new_multi_thread()
    .enable_time()
    .build()
    .expect("Failed to create tokio-runtime");
    runtime.block_on(f)
}
