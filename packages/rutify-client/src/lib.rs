use anyhow::Result;
use rutify_sdk::client::TokenResponse;
use rutify_sdk::{
    NotificationInput, NotifyEvent, NotifyItem, RutifyClient, Stats, WebSocketMessage,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 共享的客户端状态管理
#[derive(Clone)]
pub struct ClientState {
    pub client: RutifyClient,
    pub notifications: Arc<Mutex<VecDeque<NotifyItem>>>,
    pub stats: Arc<Mutex<Option<Stats>>>,
}

impl ClientState {
    pub fn new(server_url: &str) -> Self {
        Self {
            client: RutifyClient::new(server_url),
            notifications: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            stats: Arc::new(Mutex::new(None)),
        }
    }

    /// 获取所有通知
    pub async fn get_notifies(&self) -> Result<Vec<NotifyItem>> {
        let notifies = self.client.get_notifies().await?;

        // 更新本地缓存
        let mut guard = self.notifications.lock().unwrap();
        guard.clear();
        guard.extend(notifies.clone());

        Ok(notifies)
    }

    /// 获取服务器统计信息
    pub async fn get_stats(&self) -> Result<Stats> {
        let stats = self.client.get_stats().await?;

        // 更新本地缓存
        let mut guard = self.stats.lock().unwrap();
        *guard = Some(stats.clone());

        Ok(stats)
    }

    /// 发送通知
    pub async fn send_notification(&self, input: &NotificationInput) -> Result<()> {
        self.client
            .send_notification(input)
            .await
            .map_err(|e| anyhow::Error::new(e))
    }

    /// 连接WebSocket并返回消息接收器
    pub async fn connect_websocket(&self) -> Result<mpsc::UnboundedReceiver<WebSocketMessage>> {
        self.client
            .connect_websocket()
            .await
            .map_err(|e| anyhow::Error::new(e))
    }

    /// 监听WebSocket消息并更新状态
    pub async fn listen_websocket_updates(
        &self,
    ) -> Result<mpsc::UnboundedReceiver<WebSocketNotification>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let notifications = Arc::clone(&self.notifications);

        let mut ws_rx = self.connect_websocket().await?;

        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                match msg {
                    WebSocketMessage::Event(event) => {
                        // 更新本地通知缓存
                        let mut guard = notifications.lock().unwrap();
                        if guard.len() >= 100 {
                            guard.pop_front();
                        }
                        guard.push_back(NotifyItem {
                            id: 0, // Will be set by server
                            title: event.data.title.clone(),
                            notify: event.data.notify.clone(),
                            device: event.data.device.clone(),
                            received_at: event.timestamp,
                        });

                        // 发送通知
                        let _ = tx.send(WebSocketNotification::Event(event));
                    }
                    WebSocketMessage::Text(text) => {
                        let _ = tx.send(WebSocketNotification::Text(text));
                    }
                    WebSocketMessage::Error { message } => {
                        let _ = tx.send(WebSocketNotification::Error { message });
                    }
                    WebSocketMessage::Close => {
                        let _ = tx.send(WebSocketNotification::Close);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    /// 设置认证Token
    pub fn set_token(&mut self, token: &str) {
        self.client.set_token(token);
    }

    /// 清除认证Token
    pub fn clear_token(&mut self) {
        self.client.clear_token();
    }

    /// 检查是否有Token
    pub fn has_token(&self) -> bool {
        self.client.token.is_some()
    }

    /// 创建新的Token
    pub async fn create_token(&self, usage: &str, expires_in_hours: u64) -> Result<TokenResponse> {
        self.client
            .create_token(usage, expires_in_hours)
            .await
            .map_err(|e| anyhow::Error::new(e))
    }

    /// 使用Token创建客户端
    pub fn with_token(server_url: &str, token: &str) -> Self {
        let client = RutifyClient::new(server_url).with_token(token);
        Self {
            client,
            notifications: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            stats: Arc::new(Mutex::new(None)),
        }
    }
}

/// WebSocket通知类型
#[derive(Debug, Clone)]
pub enum WebSocketNotification {
    Event(NotifyEvent),
    Text(String),
    Error { message: String },
    Close,
}

/// 发送通知并监听响应的便捷方法
pub async fn send_and_listen(
    state: &ClientState,
    message: String,
    title: Option<String>,
    device: Option<String>,
) -> Result<Option<WebSocketNotification>> {
    let input = NotificationInput {
        notify: message,
        title,
        device,
    };

    // 发送通知
    state.send_notification(&input).await?;

    // 监听响应
    let mut rx = state.listen_websocket_updates().await?;

    // 等待第一个响应
    if let Some(notification) = rx.recv().await {
        Ok(Some(notification))
    } else {
        Ok(None)
    }
}

/// 健康检查
pub async fn health_check(state: &ClientState) -> Result<bool> {
    match state.get_stats().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// 格式化通知显示
pub fn format_notification(notify: &NotifyItem) -> String {
    format!(
        "{} - {} ({})\nReceived: {}",
        notify.title,
        notify.notify,
        notify.device,
        notify.received_at.format("%Y-%m-%d %H:%M:%S")
    )
}

/// 格式化统计信息显示
pub fn format_stats(stats: &Stats) -> String {
    format!(
        "Today's notifications: {}\nTotal notifications: {}\nActive devices: {}\nServer running: {}",
        stats.today_count,
        stats.total_count,
        stats.device_count,
        if stats.is_running {
            "✅ Yes"
        } else {
            "❌ No"
        }
    )
}
