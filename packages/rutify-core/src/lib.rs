use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 通知项数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyItem {
    pub id: i32,
    pub title: String,
    pub notify: String,
    pub device: String,
    pub received_at: DateTime<Utc>,
}

/// 服务器统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub today_count: i32,
    pub total_count: i32,
    pub device_count: i32,
    pub is_running: bool,
}

/// 通知输入参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInput {
    pub notify: String,
    pub title: Option<String>,
    pub device: Option<String>,
}

/// API 响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: T,
}

/// WebSocket 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyEvent {
    pub event: String,
    pub data: NotificationData,
    pub timestamp: DateTime<Utc>,
}

/// 通知数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub notify: String,
    pub title: String,
    pub device: String,
}

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// 通知事件
    Event(NotifyEvent),
    /// 纯文本消息
    Text(String),
    /// 关闭连接
    Close,
    /// 错误消息
    Error { message: String },
    /// 心跳包
    Ping,
    /// 心跳响应
    Pong,
}

/// Token 管理相关结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenItem {
    pub id: i32,
    pub token_hash: String,
    pub usage: String,
    pub created_at: DateTime<Utc>,
}

/// Token 创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub usage: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Token 创建响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub token_item: TokenItem,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: Option<i32>,
    pub name: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server_url: String,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:3000".to_string(),
            timeout_seconds: 30,
            retry_attempts: 3,
        }
    }
}

/// 错误类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RutifyError {
    /// 网络错误
    Network { message: String },
    /// API 错误
    Api { status: String, message: String },
    /// 解析错误
    Parse { message: String },
    /// 认证错误
    Auth { message: String },
    /// 配置错误
    Config { message: String },
    /// 未知错误
    Unknown { message: String },
}

impl std::fmt::Display for RutifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RutifyError::Network { message } => write!(f, "Network error: {}", message),
            RutifyError::Api { status, message } => write!(f, "API error [{}]: {}", status, message),
            RutifyError::Parse { message } => write!(f, "Parse error: {}", message),
            RutifyError::Auth { message } => write!(f, "Auth error: {}", message),
            RutifyError::Config { message } => write!(f, "Config error: {}", message),
            RutifyError::Unknown { message } => write!(f, "Unknown error: {}", message),
        }
    }
}

impl std::error::Error for RutifyError {}

/// 结果类型
pub type RutifyResult<T> = Result<T, RutifyError>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_notify_item_creation() {
        let item = NotifyItem {
            id: 1,
            title: "Test Title".to_string(),
            notify: "Test Message".to_string(),
            device: "Test Device".to_string(),
            received_at: Utc::now(),
        };

        assert_eq!(item.id, 1);
        assert_eq!(item.title, "Test Title");
        assert_eq!(item.notify, "Test Message");
        assert_eq!(item.device, "Test Device");
    }

    #[test]
    fn test_stats_creation() {
        let stats = Stats {
            today_count: 10,
            total_count: 100,
            device_count: 5,
            is_running: true,
        };

        assert_eq!(stats.today_count, 10);
        assert_eq!(stats.total_count, 100);
        assert_eq!(stats.device_count, 5);
        assert!(stats.is_running);
    }

    #[test]
    fn test_notification_input() {
        let input = NotificationInput {
            notify: "Test notification".to_string(),
            title: Some("Test Title".to_string()),
            device: Some("Test Device".to_string()),
        };

        assert_eq!(input.notify, "Test notification");
        assert_eq!(input.title, Some("Test Title".to_string()));
        assert_eq!(input.device, Some("Test Device".to_string()));
    }

    #[test]
    fn test_websocket_message_text() {
        let msg = WebSocketMessage::Text("Hello World".to_string());
        match msg {
            WebSocketMessage::Text(text) => assert_eq!(text, "Hello World"),
            _ => panic!("Expected Text message"),
        }
    }

    #[test]
    fn test_rutify_error_display() {
        let error = RutifyError::Network {
            message: "Network error".to_string(),
        };
        assert_eq!(error.to_string(), "Network error: Network error");
    }

    #[test]
    fn test_token_item_creation() {
        let token = TokenItem {
            id: 1,
            token_hash: "abc123".to_string(),
            usage: "api".to_string(),
            created_at: Utc::now(),
        };

        assert_eq!(token.id, 1);
        assert_eq!(token.token_hash, "abc123");
        assert_eq!(token.usage, "api");
    }

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo {
            id: Some(123),
            name: "Test Device".to_string(),
            last_seen: Some(Utc::now()),
            is_active: true,
        };

        assert_eq!(device.id, Some(123));
        assert_eq!(device.name, "Test Device");
        assert!(device.is_active);
    }
}
