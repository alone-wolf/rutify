use axum::Router;
use axum::routing::get;
use common_http_server_rs::{MonitoringState, metrics_endpoint, monitoring_info_endpoint};

pub(crate) fn router(monitoring: MonitoringState) -> Router {
    Router::new()
        .route(
            "/metrics",
            get({
                let monitoring = monitoring.clone();
                move || {
                    let monitoring = monitoring.clone();
                    async move { metrics_endpoint(axum::extract::State(monitoring)).await }
                }
            }),
        )
        .route(
            "/monitoring",
            get({
                let monitoring = monitoring.clone();
                move || {
                    let monitoring = monitoring.clone();
                    async move { monitoring_info_endpoint(axum::extract::State(monitoring)).await }
                }
            }),
        )
}
