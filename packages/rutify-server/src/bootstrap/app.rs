use crate::routes;
use crate::state::AppState;
use anyhow::Result;
use axum::routing::get;
use common_http_server_rs::{
    AppBuilder, AppConfig, GlobalMonitoringConfig, MiddlewareOrchestrator,
    PerformanceMonitoringConfig, ProtectionStackBuilder, ddos_presets, rate_limit_presets,
    size_limit_presets,
};
use std::sync::Arc;

pub(crate) fn app_builder(state: Arc<AppState>, app_config: AppConfig) -> Result<AppBuilder> {
    let ddos_config = ddos_presets::lenient();
    let rate_limit_config = rate_limit_presets::lenient();
    let size_limit_config = size_limit_presets::api();

    let protection_stack = ProtectionStackBuilder::new()
        .with_ddos(ddos_config.clone())
        .with_rate_limit(rate_limit_config.clone())
        .with_size_limit_content_length_only(size_limit_config.clone())
        .build()?;

    let monitor_router = routes::monitor::router(state.monitoring.clone());

    let orchestrator = MiddlewareOrchestrator::new()
        .with_app_runtime_layers(true)
        .with_monitoring_config(
            state.monitoring.clone(),
            GlobalMonitoringConfig::new().with_performance_config(
                PerformanceMonitoringConfig::new()
                    .exclude_request_count_path_prefix("/monitor")
                    .exclude_request_count_path_prefix("/health"),
            ),
        )
        .with_protection_stack(protection_stack);

    Ok(AppBuilder::new(app_config)
        .validate_ddos_config(ddos_config)
        .validate_rate_limit_config(rate_limit_config)
        .validate_size_limit_config(size_limit_config)
        .route("/", get(routes::index::handler))
        .route(
            "/ws",
            get(routes::notify::ws_handler).with_state(Arc::clone(&state)),
        )
        .nest(
            "/notify",
            routes::notify::router().with_state(Arc::clone(&state)),
        )
        .nest(
            "/api",
            routes::api::router(Arc::clone(&state)).with_state(Arc::clone(&state)),
        )
        .nest(
            "/auth",
            routes::auth::router(Arc::clone(&state)).with_state(Arc::clone(&state)),
        )
        .nest("/monitor", monitor_router)
        .with_orchestrator(orchestrator))
}
