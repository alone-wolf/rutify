use crate::SdkResult;
use crate::auth::{
    CreateTokenRequest, CreateTokenResponse, LoginRequest, LoginResponse, RegisterRequest,
    TokenInfo,
};
use crate::error::*;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use rutify_core::*;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Clone)]
pub struct RutifyClient {
    client: Client,
    pub base_url: String,
    pub timeout: Duration,
    pub token: Option<String>,
    pub user_token: Option<String>, // 用户JWT token
}

impl RutifyClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(30),
            token: None,
            user_token: None,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    pub fn with_user_token(mut self, user_token: &str) -> Self {
        self.user_token = Some(user_token.to_string());
        self
    }

    pub fn set_user_token(&mut self, user_token: &str) {
        self.user_token = Some(user_token.to_string());
    }

    pub fn clear_user_token(&mut self) {
        self.user_token = None;
    }

    pub fn has_user_token(&self) -> bool {
        self.user_token.is_some()
    }

    pub fn set_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }

    pub fn clear_token(&mut self) {
        self.token = None;
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn api_request<T>(&self, endpoint: &str) -> SdkResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!(
            "{}/{}/{}",
            self.base_url.trim_end_matches('/'),
            "api",
            endpoint.trim_start_matches('/')
        );
        let mut request = self.client.get(&url).timeout(self.timeout);

        // 添加Authorization头如果有token
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        let response = response.error_for_status()?;
        let api_response: ApiResponse<T> = response.json().await?;

        if api_response.status != "ok" {
            return Err(SdkError::ApiError {
                status: api_response.status,
            });
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
        let mut request = self.client.post(&url).timeout(self.timeout).json(input);

        // 添加Authorization头如果有token
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        response.error_for_status()?;
        Ok(())
    }

    pub async fn connect_websocket(
        &self,
    ) -> SdkResult<tokio::sync::mpsc::UnboundedReceiver<WebSocketMessage>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut ws_url = format!(
            "{}/ws",
            self.base_url.trim_end_matches('/').replace("http", "ws")
        );

        // 添加token参数如果有token
        if let Some(token) = &self.token {
            ws_url = format!("{}?token={}", ws_url, token);
        }

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
                                let _ = tx.send(WebSocketMessage::Error {
                                    message: e.to_string(),
                                });
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
        let mut ws_url = format!(
            "{}/ws",
            self.base_url.trim_end_matches('/').replace("http", "ws")
        );

        // 添加token参数如果有token
        if let Some(token) = &self.token {
            ws_url = format!("{}?token={}", ws_url, token);
        }

        match connect_async(&ws_url).await {
            Ok((mut ws_stream, _)) => {
                ws_stream
                    .send(Message::Text(message.to_string().into()))
                    .await
                    .map_err(|e| SdkError::NetworkError(e.to_string()))?;
                Ok(())
            }
            Err(e) => Err(SdkError::NetworkError(e.to_string())),
        }
    }

    /// 创建新的Token
    pub async fn create_token(
        &self,
        usage: &str,
        expires_in_hours: u64,
    ) -> SdkResult<TokenResponse> {
        let url = format!("{}/auth/tokens", self.base_url.trim_end_matches('/'));
        let request_body = serde_json::json!({
            "usage": usage,
            "expires_in_hours": expires_in_hours
        });

        let mut request = self
            .client
            .post(&url)
            .timeout(self.timeout)
            .json(&request_body);

        if let Some(user_token) = &self.user_token {
            request = request.header("Authorization", format!("Bearer {}", user_token));
        }

        let response = request.send().await?;

        let response = response.error_for_status()?;
        let token_response: TokenResponse = response.json().await?;

        Ok(token_response)
    }

    // ========== 用户认证方法 ==========

    /// 用户注册
    pub async fn register(&self, request: &RegisterRequest) -> SdkResult<()> {
        let url = format!("{}/auth/register", self.base_url);
        let response = self
            .client
            .post(&url)
            .timeout(self.timeout)
            .json(request)
            .send()
            .await?;

        response.error_for_status()?;
        Ok(())
    }

    /// 用户登录
    pub async fn login(&self, request: &LoginRequest) -> SdkResult<LoginResponse> {
        let url = format!("{}/auth/login", self.base_url);
        let response = self
            .client
            .post(&url)
            .timeout(self.timeout)
            .json(request)
            .send()
            .await?;

        let response = response.error_for_status()?;
        let login_response: LoginResponse = response.json().await?;
        Ok(login_response)
    }

    /// 获取用户信息
    pub async fn get_user_profile(&self) -> SdkResult<TokenInfo> {
        let url = format!("{}/auth/profile", self.base_url);
        let mut request = self.client.get(&url).timeout(self.timeout);

        // 添加用户JWT token
        if let Some(user_token) = &self.user_token {
            request = request.header("Authorization", format!("Bearer {}", user_token));
        }

        let response = request.send().await?;
        let response = response.error_for_status()?;
        let user_info: TokenInfo = response.json().await?;
        Ok(user_info)
    }

    /// 创建通知Token
    pub async fn create_notify_token(
        &self,
        request: &CreateTokenRequest,
    ) -> SdkResult<CreateTokenResponse> {
        let url = format!("{}/auth/tokens", self.base_url);
        let mut request_builder = self.client.post(&url).timeout(self.timeout).json(request);

        // 添加用户JWT token
        if let Some(user_token) = &self.user_token {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", user_token));
        }

        let response = request_builder.send().await?;
        let response = response.error_for_status()?;
        let token_response: CreateTokenResponse = response.json().await?;
        Ok(token_response)
    }

    /// 获取用户的Token列表
    pub async fn get_user_tokens(&self) -> SdkResult<Vec<TokenInfo>> {
        let url = format!("{}/auth/tokens", self.base_url);
        let mut request = self.client.get(&url).timeout(self.timeout);

        // 添加用户JWT token
        if let Some(user_token) = &self.user_token {
            request = request.header("Authorization", format!("Bearer {}", user_token));
        }

        let response = request.send().await?;
        let response = response.error_for_status()?;
        let tokens: Vec<TokenInfo> = response.json().await?;
        Ok(tokens)
    }

    /// 删除用户Token
    pub async fn delete_user_token(&self, token_id: i32) -> SdkResult<()> {
        let url = format!("{}/auth/tokens/{}", self.base_url, token_id);
        let mut request = self.client.delete(&url).timeout(self.timeout);

        // 添加用户JWT token
        if let Some(user_token) = &self.user_token {
            request = request.header("Authorization", format!("Bearer {}", user_token));
        }

        let response = request.send().await?;
        response.error_for_status()?;
        Ok(())
    }

    /// 便捷方法：登录并自动设置用户token
    pub async fn login_and_set_token(
        &mut self,
        username: &str,
        password: &str,
    ) -> SdkResult<LoginResponse> {
        let login_request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self.login(&login_request).await?;
        self.set_user_token(&response.jwt_token);
        Ok(response)
    }

    /// 便捷方法：创建通知token并自动设置
    pub async fn create_and_set_notify_token(
        &mut self,
        usage: &str,
        device_info: Option<String>,
    ) -> SdkResult<CreateTokenResponse> {
        let token_request = CreateTokenRequest {
            usage: usage.to_string(),
            expires_in_hours: Some(24),
            device_info,
        };

        let response = self.create_notify_token(&token_request).await?;
        self.set_token(&response.token);
        Ok(response)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub token_id: String,
    pub usage: String,
    pub expires_at: String,
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
        let client =
            RutifyClient::new("http://localhost:3000").with_timeout(Duration::from_secs(60));
        assert_eq!(client.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_sdk_error_display() {
        let error = SdkError::NetworkError("Test errors".to_string());
        assert_eq!(error.to_string(), "Network errors: Test errors");
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
        let client =
            RutifyClient::new("http://localhost:3000").with_timeout(Duration::from_millis(500));

        assert_eq!(client.timeout, Duration::from_millis(500));
    }
}
