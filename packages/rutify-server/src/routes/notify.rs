use crate::error::AppError;
use rutify_core::{NotificationData, NotificationInput, NotifyEvent};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

const DEFAULT_TITLE: &str = "default title";
const DEFAULT_DEVICE: &str = "default device";
pub(crate) async fn list_notifies_handler() -> Result<impl IntoResponse, AppError> {
    let data = vec![
        serde_json::json!({
            "id": 1,
            "title": "Service Started",
            "notify": "Rutify started successfully",
            "device": "server-1",
            "received_at": "2026-02-03 09:00:12"
        }),
        serde_json::json!({
            "id": 2,
            "title": "Login Success",
            "notify": "User wolf logged in successfully",
            "device": "macbook-pro",
            "received_at": "2026-02-03 09:03:45"
        }),
        serde_json::json!({
            "id": 3,
            "title": "Task Completed",
            "notify": "Message forwarding task finished",
            "device": "worker-2",
            "received_at": "2026-02-03 09:08:27"
        }),
        serde_json::json!({
            "id": 4,
            "title": "Warning",
            "notify": "WebSocket disconnected briefly and recovered",
            "device": "gateway",
            "received_at": "2026-02-03 09:12:09"
        }),
        serde_json::json!({
            "id": 5,
            "title": "Broadcast Done",
            "notify": "Notification sent to 5 clients",
            "device": "server-1",
            "received_at": "2026-02-03 09:15:33"
        }),
    ];

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "data": data
        })),
    ))
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
        event: "notify".to_string(),
        data,
        timestamp: chrono::Utc::now(),
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
