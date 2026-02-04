use std::sync::Arc;
use axum::{Router, middleware};
use axum::routing::{get, post};
use tower_http::LatencyUnit;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use crate::auth;
use crate::routes::{notify, root, ws, stats};
use crate::auth::auth_middleware;
use crate::state::AppState;

pub(crate) fn axum_app(state:Arc<AppState>) -> Router {
    Router::new()
        // 公开路由 (不需要授权)
        .route("/", get(root::root))
        // Token 管理路由 (不需要授权，用于创建token)
        .route("/auth/token", post(auth::create_token))
        // 需要授权的路由
        .route("/notify", get(notify::receive_notify_get_handler))
        .route("/notify", post(notify::receive_notify_post_handler))
        .route("/api/notifies", get(notify::list_notifies_handler))
        .route("/api/stats", get(stats::stats_handler))
        .route("/ws", get(ws::ws_handler))
        // 应用授权中间件到需要授权的路由
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            auth_middleware
        ))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(LatencyUnit::Millis))
        )
}
