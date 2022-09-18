use axum::routing::get;
use axum::{response, Extension, Router};

use agner_actors::{System, SystemConfig};

pub fn add_routes(router: Router) -> Router {
    router.route("/system/config", get(system_config))
}

async fn system_config(Extension(system): Extension<System>) -> response::Json<SystemConfig> {
    response::Json(system.config().to_owned())
}
