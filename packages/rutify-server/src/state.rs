use sea_orm::DatabaseConnection;
use tokio::sync::broadcast;
use rutify_core::NotifyEvent;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) tx: broadcast::Sender<NotifyEvent>,
}