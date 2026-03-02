use crate::state::AppState;
use axum::Router;
use std::sync::Arc;

mod notifies;
mod stats;

pub(crate) fn router(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .nest("/notifies", notifies::router())
        .nest("/stats", stats::router())
        // Backward-compatible alias.
        .nest("/states", stats::router())
}
