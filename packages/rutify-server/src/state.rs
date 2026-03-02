use common_http_server_rs::MonitoringState;
use rutify_core::NotifyEvent;
use sea_orm::DatabaseConnection;
use tokio::sync::broadcast;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) tx: broadcast::Sender<NotifyEvent>,
    pub(crate) monitoring: MonitoringState,
}
