use std::net::SocketAddr;

use agner_actors::{BoxError, System};
use axum::{Extension, Router, Server};

mod actors;
mod system;

pub async fn run(system: System, bind_addr: SocketAddr) -> Result<(), BoxError> {
    let router = Router::new();

    let router = system::add_routes(router);
    let router = actors::routes(router);

    let router = router.layer(Extension(system));

    let () = Server::bind(&bind_addr).serve(router.into_make_service()).await?;
    Ok(())
}
