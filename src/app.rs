use std::sync::Arc;
use axum::Router;
use axum::routing::{get, post};
use crate::routes::{notify, root, ws};
use crate::state::AppState;

pub(crate) fn axum_app(state:Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root::root))
        .route("/notify", get(notify::receive_notify_get_handler))
        .route("/notify", post(notify::receive_notify_post_handler))
        .route("/api/notifies",get(notify::list_notifies_handler))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}