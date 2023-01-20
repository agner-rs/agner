use agner_utils::future_timeout_ext::FutureTimeoutExt;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get};
use axum::{response, Extension, Json, Router};

use agner_actors::{ActorID, Exit, System};
use agner_sup::common::ParentActor;

use futures::StreamExt;

mod exit_reason_serde;

pub fn routes(router: Router) -> Router {
    router
        .route("/actors", get(actors_list))
        .route("/actors/:actor_id", get(actors_actor_info))
        .route("/actors/:actor_id", delete(actors_actor_exit))
}

async fn actors_list(Extension(system): Extension<System>) -> response::Json<Vec<ActorID>> {
    response::Json(system.all_actors().collect().await)
}

async fn actors_actor_info(
    Extension(system): Extension<System>,
    Path(actor_id): Path<ActorID>,
) -> response::Json<Option<serde_json::Value>> {
    let actor_info = system.actor_info(actor_id).await;
    let parent_actor_opt = system.get_data::<ParentActor>(actor_id).await.map(|pa| pa.0);

    let actor_info = actor_info.map(|actor_info| {
        let mut actor_info = serde_json::to_value(&actor_info).expect("Json failed to serialize");
        if let serde_json::Value::Object(fields) = &mut actor_info {
            if let Some(parent_actor_id) = parent_actor_opt {
                fields.insert(
                    "parent_actor".to_owned(),
                    serde_json::to_value(parent_actor_id).expect("Failed to serialize"),
                );
            }
        }
        actor_info
    });

    response::Json(actor_info)
}

async fn actors_actor_exit(
    Extension(system): Extension<System>,
    Path(actor_id): Path<ActorID>,
    Json(exit_reason): Json<exit_reason_serde::ExitSerde>,
) -> impl IntoResponse {
    let exit_reason: Exit = exit_reason.into();
    system.exit(actor_id, exit_reason).await;

    match system.wait(actor_id).timeout(system.config().actor_termination_timeout).await {
        Ok(exit_reason) => {
            let exit_reason = exit_reason_serde::ExitSerde::from(exit_reason);
            let response = Json(exit_reason);
            (StatusCode::ACCEPTED, response).into_response()
        },

        Err(_) => StatusCode::REQUEST_TIMEOUT.into_response(),
    }
}
