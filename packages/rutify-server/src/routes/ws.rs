use std::sync::Arc;
use axum::extract::{State, WebSocketUpgrade, Query};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use crate::state::AppState;
use crate::auth::{verify_ws_token, check_token_exists};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: String,
}

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    // 验证 WebSocket token
    match verify_ws_token(&query.token, &state).await {
        Ok(claims) => {
            info!("WebSocket connection authorized for token usage: {}", claims.usage);
            
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
                        error!("Database error during WebSocket token verification: {}", e);
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

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, claims: crate::auth::Claims) {
    let mut rx = state.tx.subscribe();
    
    info!("WebSocket connection established for usage: {}", claims.usage);

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
                        error!(error = %err, "websocket receive error for usage: {}", claims.usage);
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
                                error!(error = %err, "websocket serialize error for usage: {}", claims.usage);
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