use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyItemData {
    pub id: i32,
    pub title: String,
    pub notify: String,
    pub device: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub today_count: i32,
    pub total_count: i32,
    pub device_count: i32,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInput {
    pub notify: String,
    pub title: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyEvent {
    pub event: String,
    pub data: NotificationData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub notify: String,
    pub title: String,
    pub device: String,
}

#[derive(Debug, Clone)]
pub enum NotificationMessage {
    Event(NotifyEvent),
    Text(String),
    Close,
    Error(String),
}
