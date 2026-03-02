use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use rutify_core::Stats;
use sea_orm::EntityTrait;
use std::collections::HashSet;
use std::sync::Arc;

pub(crate) fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(stats_handler))
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let notifies = crate::db::notifies::Entity::find().all(&state.db).await?;
    let today = chrono::Utc::now().date_naive();

    let today_count = notifies
        .iter()
        .filter(|item| item.received_at.date_naive() == today)
        .count() as i32;

    let device_count = notifies
        .iter()
        .filter_map(|item| item.device.clone())
        .collect::<HashSet<String>>()
        .len() as i32;

    let data = Stats {
        today_count,
        total_count: notifies.len() as i32,
        device_count,
        is_running: true,
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "data": data
        })),
    ))
}
