use crate::error::AppError;
use crate::payload::{NotificationData, NotificationInput, NotifyEvent};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

const DEFAULT_TITLE: &str = "default title";
const DEFAULT_DEVICE: &str = "default device";
pub(crate) async fn list_notifies_handler() -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

pub(crate) async fn receive_notify_get_handler(
    State(state): State<Arc<AppState>>,
    Query(payload): Query<NotificationInput>,
) -> Result<impl IntoResponse, AppError> {
    receive_notify_logic(state, payload).await;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

pub(crate) async fn receive_notify_post_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NotificationInput>,
) -> Result<impl IntoResponse, AppError> {
    receive_notify_logic(state, payload).await;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

async fn receive_notify_logic(state: Arc<AppState>, payload: NotificationInput) {
    let db = &state.db;
    let tx = &state.tx;
    let data = normalize_notification(payload);
    crate::db::notifies::insert_new_notify(db, data.clone()).await;
    let event = NotifyEvent {
        event: "notify",
        data,
    };
    let _ = tx.send(event);
}

fn normalize_notification(payload: NotificationInput) -> NotificationData {
    NotificationData {
        notify: payload.notify,
        title: payload.title.unwrap_or_else(|| DEFAULT_TITLE.to_string()),
        device: payload.device.unwrap_or_else(|| DEFAULT_DEVICE.to_string()),
    }
}
