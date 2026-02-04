use std::sync::Arc;
use axum::Router;
use axum::routing::{get, post};
use tower_http::LatencyUnit;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use crate::routes::{notify, root, ws, stats};
use crate::state::AppState;

pub(crate) fn axum_app(state:Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root::root))
        .route("/notify", get(notify::receive_notify_get_handler))
        .route("/notify", post(notify::receive_notify_post_handler))
        .route("/api/notifies",get(notify::list_notifies_handler))
        .route("/api/stats", get(stats::stats_handler))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(LatencyUnit::Millis))
        )
}
