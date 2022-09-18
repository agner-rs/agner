use axum::extract::Path;
use axum::{response, Extension, Router};

use agner_actors::{ActorID, ActorInfo, System};

use futures::StreamExt;

mod exit_reason_serde;

pub fn routes(router: Router) -> Router {
    router
}

async fn actors_list(Extension(system): Extension<System>) -> response::Json<Vec<ActorID>> {
    response::Json(system.all_actors().collect().await)
}

async fn actors_actor_info(
    Extension(system): Extension<System>,
    Path(actor_id): Path<ActorID>,
) -> response::Json<Option<ActorInfo>> {
    response::Json(system.actor_info(actor_id).await)
}

async fn actors_actor_exit(
    Extension(system): Extension<System>,
    Path(actor_id): Path<ActorID>,
) -> response::Json<()> {
    response::Json(())
}
