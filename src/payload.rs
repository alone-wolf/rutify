use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct NotificationInput {
    pub(crate) notify: String,
    pub(crate) title: Option<String>,
    pub(crate) device: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct NotificationData {
    pub(crate) notify: String,
    pub(crate) title: String,
    pub(crate) device: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct NotifyEvent {
    pub(crate) event: &'static str,
    pub(crate) data: NotificationData,
}