use crate::error::AppError;
use crate::services::auth::auth::{check_token_exists, verify_ws_token};
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rutify_core::{NotificationData, NotificationInput, NotifyEvent};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

const DEFAULT_TITLE: &str = "default title";
const DEFAULT_DEVICE: &str = "default device";

pub(crate) fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(receive_notify_get_handler))
        .route("/", post(receive_notify_post_handler))
        .route("/ws", get(ws_handler))
}

async fn receive_notify_get_handler(
    State(state): State<Arc<AppState>>,
    Query(payload): Query<NotificationInput>,
) -> Result<impl IntoResponse, AppError> {
    receive_notify_logic(state, payload).await;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

async fn receive_notify_post_handler(
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

#[derive(Debug, Deserialize)]
pub(crate) struct WsQuery {
    token: String,
}

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    match verify_ws_token(&query.token, &state).await {
        Ok(claims) => {
            info!(
                "WebSocket connection authorized for token usage: {}",
                claims.usage
            );

            // 异步验证 token 是否在数据库中存在
            let state_clone = Arc::clone(&state);
            let token_clone = query.token.clone();
            let _claims_clone = claims.clone();

            tokio::spawn(async move {
                match check_token_exists(&token_clone, &state_clone).await {
                    Ok(true) => {
                        info!("Token verified in database for WebSocket connection");
                    }
                    Ok(false) => {
                        warn!("Token not found in database for WebSocket connection");
                    }
                    Err(e) => {
                        error!("Database errors during WebSocket token verification: {}", e);
                    }
                }
            });

            ws.on_upgrade(move |socket| handle_socket(socket, state, claims))
        }
        Err(e) => {
            error!("WebSocket authorization failed: {}", e);
            // 返回错误响应而不是升级连接
            axum::response::Response::builder()
                .status(axum::http::StatusCode::UNAUTHORIZED)
                .body(axum::body::Body::from("Unauthorized"))
                .unwrap()
                .into_response()
        }
    }
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    claims: crate::services::auth::auth::TokenClaims,
) {
    let mut rx = state.tx.subscribe();

    info!(
        "WebSocket connection established for usage: {}",
        claims.usage
    );

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WebSocket connection closed for usage: {}", claims.usage);
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        error!(error = %err, "websocket receive errors for usage: {}", claims.usage);
                        break;
                    }
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        match serde_json::to_string(&event) {
                            Ok(text) => {
                                if socket.send(Message::Text(text.into())).await.is_err() {
                                    warn!("Failed to send message to WebSocket for usage: {}", claims.usage);
                                    break;
                                }
                            }
                            Err(err) => {
                                error!(error = %err, "websocket serialize errors for usage: {}", claims.usage);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Broadcast channel closed for usage: {}", claims.usage);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("WebSocket client lagged for usage: {}", claims.usage);
                    }
                }
            }
        }
    }
}
