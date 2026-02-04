use crate::sdk::{error::*, types::*};
use reqwest::Client;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

#[derive(Clone)]
pub struct RutifyClient {
    client: Client,
    base_url: String,
    ws_client: Arc<Mutex<Option<WebSocketClient>>>,
}

struct WebSocketClient {
    sender: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    _task: tokio::task::JoinHandle<()>,
}

impl RutifyClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self { 
            client, 
            base_url,
            ws_client: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> SdkResult<Self> {
        let base_url = base_url.into();
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| SdkError::NetworkError(e.to_string()))?;
        
        Ok(Self { 
            client, 
            base_url,
            ws_client: Arc::new(Mutex::new(None)),
        })
    }

    async fn api_request<T>(&self, endpoint: &str) -> SdkResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));
        let response = self.client.get(&url).send().await?;
        let response = response.error_for_status()?;
        let api_response: ApiResponse<T> = response.json().await?;
        
        if api_response.status != "ok" {
            return Err(SdkError::ApiError { status: api_response.status });
        }
        
        Ok(api_response.data)
    }

    pub async fn get_notifies(&self) -> SdkResult<Vec<NotifyItemData>> {
        self.api_request("api/notifies").await
    }

    pub async fn get_stats(&self) -> SdkResult<Stats> {
        self.api_request("api/stats").await
    }

    pub async fn send_notify(&self, input: &NotificationInput) -> SdkResult<()> {
        let url = format!("{}/notify", self.base_url.trim_end_matches('/'));
        let response = self.client.post(&url).json(input).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    pub async fn connect_websocket<F>(&self, callback: F) -> SdkResult<()>
    where
        F: Fn(NotificationMessage) + Send + Sync + 'static,
    {
        let ws_url = format!("{}/ws", self.base_url.trim_end_matches('/').replace("http", "ws"));
        
        let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| SdkError::NetworkError(e.to_string()))?;
        let (sender, receiver) = ws_stream.split();
        
        let callback = Arc::new(callback);
        
        let task = tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<NotifyEvent>(&text.to_string()) {
                            Ok(event) => callback(NotificationMessage::Event(event)),
                            Err(_) => callback(NotificationMessage::Text(text.to_string())),
                        }
                    }
                    Ok(Message::Close(_)) => {
                        callback(NotificationMessage::Close);
                        break;
                    }
                    Ok(Message::Binary(data)) => {
                        if let Ok(text) = String::from_utf8(data.to_vec()) {
                            callback(NotificationMessage::Text(text));
                        }
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => {
                        // Ignore ping/pong/frame messages
                    }
                    Err(e) => {
                        callback(NotificationMessage::Error(e.to_string()));
                        break;
                    }
                }
            }
        });
        
        let ws_client = WebSocketClient {
            sender,
            _task: task,
        };
        
        let mut ws_lock = self.ws_client.lock().await;
        *ws_lock = Some(ws_client);
        
        Ok(())
    }

    pub async fn disconnect_websocket(&self) -> SdkResult<()> {
        let mut ws_lock = self.ws_client.lock().await;
        if let Some(mut ws_client) = ws_lock.take() {
            let _ = ws_client.sender.send(Message::Close(None)).await;
        }
        Ok(())
    }

    pub async fn is_websocket_connected(&self) -> bool {
        let ws_lock = self.ws_client.lock().await;
        ws_lock.is_some()
    }

    pub async fn send_websocket_message(&self, message: &str) -> SdkResult<()> {
        let mut ws_lock = self.ws_client.lock().await;
        if let Some(ws_client) = ws_lock.as_mut() {
            ws_client.sender.send(Message::Text(message.to_string().into())).await
                .map_err(|e| SdkError::NetworkError(e.to_string()))?;
        } else {
            return Err(SdkError::NetworkError("WebSocket not connected".to_string()));
        }
        Ok(())
    }
}
