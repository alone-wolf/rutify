use std::sync::Arc;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use tokio::sync::broadcast;
use tracing::error;
use crate::state::AppState;

pub(crate) async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        error!(error = %err, "websocket receive error");
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
                                    break;
                                }
                            }
                            Err(err) => {
                                error!(error = %err, "websocket serialize error");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        }
    }
}