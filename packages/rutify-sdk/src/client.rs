use crate::error::*;
use crate::SdkResult;
use rutify_core::*;
use reqwest::Client;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

#[derive(Clone)]
pub struct RutifyClient {
    client: Client,
    pub base_url: String,
    pub timeout: Duration,
}

impl RutifyClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn api_request<T>(&self, endpoint: &str) -> SdkResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}/{}", self.base_url.trim_end_matches('/'), "api", endpoint.trim_start_matches('/'));
        let response = self.client.get(&url).timeout(self.timeout).send().await?;
        let response = response.error_for_status()?;
        let api_response: ApiResponse<T> = response.json().await?;
        
        if api_response.status != "ok" {
            return Err(SdkError::ApiError { status: api_response.status });
        }
        
        Ok(api_response.data)
    }

    pub async fn get_notifies(&self) -> SdkResult<Vec<NotifyItem>> {
        self.api_request("notifies").await
    }

    pub async fn get_stats(&self) -> SdkResult<Stats> {
        self.api_request("stats").await
    }

    pub async fn send_notification(&self, input: &NotificationInput) -> SdkResult<()> {
        let url = format!("{}/notify", self.base_url.trim_end_matches('/'));
        let response = self.client.post(&url).timeout(self.timeout).json(input).send().await?;
        response.error_for_status()?;
        Ok(())
    }

    pub async fn connect_websocket(&self) -> SdkResult<tokio::sync::mpsc::UnboundedReceiver<WebSocketMessage>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let ws_url = format!("{}/ws", self.base_url.trim_end_matches('/').replace("http", "ws"));
        
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                
                // Handle incoming messages
                tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(event) = serde_json::from_str::<NotifyEvent>(&text) {
                                    let _ = tx.send(WebSocketMessage::Event(event));
                                } else {
                                    let _ = tx.send(WebSocketMessage::Text(text.to_string()));
                                }
                            }
                            Ok(Message::Binary(data)) => {
                                if let Ok(text) = String::from_utf8(data.to_vec()) {
                                    if let Ok(event) = serde_json::from_str::<NotifyEvent>(&text) {
                                        let _ = tx.send(WebSocketMessage::Event(event));
                                    } else {
                                        let _ = tx.send(WebSocketMessage::Text(text));
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                let _ = tx.send(WebSocketMessage::Close);
                                break;
                            }
                            Ok(Message::Ping(_)) => {
                                // Respond to ping with pong
                                if let Err(e) = write.send(Message::Pong(vec![].into())).await {
                                    eprintln!("Failed to send pong: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(WebSocketMessage::Error { message: e.to_string() });
                                break;
                            }
                            _ => {}
                        }
                    }
                });
                
                Ok(rx)
            }
            Err(e) => Err(SdkError::NetworkError(e.to_string())),
        }
    }

    pub async fn send_websocket_message(&self, message: &str) -> SdkResult<()> {
        let ws_url = format!("{}/ws", self.base_url.trim_end_matches('/').replace("http", "ws"));
        
        match connect_async(&ws_url).await {
            Ok((mut ws_stream, _)) => {
                ws_stream.send(Message::Text(message.to_string().into())).await
                    .map_err(|e| SdkError::NetworkError(e.to_string()))?;
                Ok(())
            }
            Err(e) => Err(SdkError::NetworkError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RutifyClient::new("http://localhost:3000");
        assert_eq!(client.base_url, "http://localhost:3000");
    }

    #[tokio::test]
    async fn test_client_with_timeout() {
        let client = RutifyClient::new("http://localhost:3000")
            .with_timeout(Duration::from_secs(60));
        assert_eq!(client.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_sdk_error_display() {
        let error = SdkError::NetworkError("Test error".to_string());
        assert_eq!(error.to_string(), "Network error: Test error");
    }

    #[test]
    fn test_sdk_result_type() {
        fn returns_success() -> SdkResult<String> {
            Ok("success".to_string())
        }

        fn returns_error() -> SdkResult<String> {
            Err(SdkError::NetworkError("test".to_string()))
        }

        assert!(returns_success().is_ok());
        assert!(returns_error().is_err());
    }

    #[tokio::test]
    async fn test_client_url_trimming() {
        let client = RutifyClient::new("http://localhost:3000/");
        assert_eq!(client.base_url, "http://localhost:3000");
        
        // trim_end_matches removes all trailing slashes
        let client = RutifyClient::new("http://localhost:3000//");
        assert_eq!(client.base_url, "http://localhost:3000");
        
        let client = RutifyClient::new("http://localhost:3000///");
        assert_eq!(client.base_url, "http://localhost:3000");
    }

    #[tokio::test]
    async fn test_timeout_configuration() {
        let client = RutifyClient::new("http://localhost:3000")
            .with_timeout(Duration::from_millis(500));
        
        assert_eq!(client.timeout, Duration::from_millis(500));
    }
}
