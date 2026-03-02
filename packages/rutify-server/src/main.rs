mod bootstrap;
mod db;
mod error;
mod routes;
mod services;
mod state;

slint::include_modules!();

use crate::state::AppState;
use clap::Parser;
use common_http_server_rs::{MonitoringState, Server, setup_metrics_recorder};
use dotenvy::dotenv;
use rutify_core::NotifyItem as CoreNotifyItem;
use rutify_sdk::RutifyClient;
use sea_orm::Database;
use slint::{ModelRc, VecModel};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::warn;

#[derive(clap::Parser)]
struct CliArgs {
    #[clap(long)]
    ui: bool,
}

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    println!("ui:{}", args.ui);
    match args.ui {
        true => run_with_ui()?,
        false => run_cli_only()?,
    }

    Ok(())
}

fn run_cli_only() -> anyhow::Result<()> {
    dotenv().ok();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { rutify_service().await })?;

    Ok(())
}

fn run_with_ui() -> anyhow::Result<()> {
    dotenv().ok();

    let ui = AppWindow::new()?;
    let rt = tokio::runtime::Runtime::new()?;
    let rt_handle = rt.handle().clone();
    let weak_ui = ui.as_weak();
    let service_addr = resolve_service_addr();
    let sdk_client = RutifyClient::new(&service_addr);
    let cached_notifies: Arc<Mutex<Vec<CoreNotifyItem>>> = Arc::new(Mutex::new(Vec::new()));
    ui.set_service_addr(service_addr.clone().into());

    // 启动服务器
    let _server_handle = rt_handle.spawn(async move {
        if let Err(e) = rutify_service().await {
            tracing::error!("Server failed to start: {}", e);
        }
        slint::invoke_from_event_loop(move || if let Some(_ui) = weak_ui.upgrade() {}).ok();
    });

    // 等待一小段时间让服务器启动
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let search_cache = Arc::clone(&cached_notifies);
    let search_ui = ui.as_weak();
    ui.on_search_notifies(move |text| {
        let query = text.to_lowercase();
        let items = {
            let guard = search_cache.lock().unwrap();
            guard.clone()
        };
        let filtered = if query.is_empty() {
            items
        } else {
            items
                .into_iter()
                .filter(|item| {
                    item.title.to_lowercase().contains(&query)
                        || item.notify.to_lowercase().contains(&query)
                        || item.device.to_lowercase().contains(&query)
                })
                .collect()
        };
        if let Some(ui) = search_ui.upgrade() {
            ui.set_all_notifies(notify_model(&filtered));
        }
    });

    let refresh_handle = rt_handle.clone();
    let refresh_sdk_client = sdk_client.clone();
    let refresh_ui = ui.as_weak();
    let refresh_cache = Arc::clone(&cached_notifies);
    ui.on_refresh_notifies(move || {
        let sdk_client = refresh_sdk_client.clone();
        let refresh_ui = refresh_ui.clone();
        let refresh_cache = Arc::clone(&refresh_cache);
        refresh_handle.spawn(async move {
            match sdk_client.get_notifies().await {
                Ok(items) => apply_notifies_to_ui(refresh_ui, refresh_cache, items),
                Err(err) => warn!("failed to refresh notifies: {err}"),
            }
        });
    });

    let initial_sdk_client = sdk_client.clone();
    let initial_ui = ui.as_weak();
    let initial_cache = Arc::clone(&cached_notifies);
    rt_handle.spawn(async move {
        match initial_sdk_client.get_notifies().await {
            Ok(items) => apply_notifies_to_ui(initial_ui, initial_cache, items),
            Err(err) => warn!("failed to load notifies: {err}"),
        }
    });

    let stats_sdk_client = sdk_client.clone();
    let stats_ui = ui.as_weak();
    rt_handle.spawn(async move {
        match stats_sdk_client.get_stats().await {
            Ok(stats) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = stats_ui.upgrade() {
                        ui.set_stats(StatData {
                            today_count: stats.today_count.into(),
                            total_count: stats.total_count.into(),
                            device_count: stats.device_count.into(),
                            is_running: stats.is_running,
                        });
                    }
                });
            }
            Err(err) => warn!("failed to load stats: {err}"),
        }
    });

    ui.run()?;
    Ok(())
}

fn resolve_service_addr() -> String {
    let addr = std::env::var("RUTIFY_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    format!("http://{}", addr.replace("0.0.0.0", "127.0.0.1"))
}

fn notify_model(items: &[CoreNotifyItem]) -> ModelRc<NotifyItem> {
    let converted: Vec<NotifyItem> = items
        .iter()
        .map(|item| NotifyItem {
            id: item.id,
            title: item.title.clone().into(),
            notify: item.notify.clone().into(),
            device: item.device.clone().into(),
            received_at: item
                .received_at
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .into(),
        })
        .collect();
    ModelRc::new(VecModel::from(converted))
}

fn apply_notifies_to_ui(
    ui: slint::Weak<AppWindow>,
    cache: Arc<Mutex<Vec<CoreNotifyItem>>>,
    items: Vec<CoreNotifyItem>,
) {
    {
        let mut guard = cache.lock().unwrap();
        *guard = items.clone();
    }

    let _cache_clone = Arc::clone(&cache);
    let items_clone = items.clone();

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            let recent: Vec<CoreNotifyItem> = items_clone.iter().take(5).cloned().collect();
            ui.set_all_notifies(notify_model(&items));
            ui.set_recent_notifies(notify_model(&recent));
        }
    });
}

async fn rutify_service() -> anyhow::Result<()> {
    let db_url = std::env::var("RUTIFY_DB_URL")
        .unwrap_or_else(|_| "sqlite://rutify.db?mode=rwc".to_string());
    let db_cnn = Database::connect(&db_url).await?;
    db::initialize::initial(&db_cnn).await;

    let monitoring = MonitoringState::new();
    setup_metrics_recorder(monitoring.clone());

    let (tx, _) = broadcast::channel(200);
    let state = Arc::new(AppState {
        db: db_cnn,
        tx,
        monitoring,
    });

    let app_config = bootstrap::config::app_config_from_env();
    let app_builder = bootstrap::app::app_builder(state, app_config)?;
    let server_config = bootstrap::config::server_config_from_env()?;
    let server = Server::new(server_config, app_builder);

    server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("failed to start server: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use slint::Model;

    #[tokio::test]
    async fn test_database_connection() {
        let db_url = "sqlite::memory:";
        let result = Database::connect(db_url).await;

        assert!(result.is_ok());
        let db = result.unwrap();
        // Test that we can ping the database
        let ping_result = db.ping().await;
        assert!(ping_result.is_ok());
    }

    #[test]
    fn test_resolve_service_addr_default() {
        // Test the function with default behavior
        let addr = resolve_service_addr();
        // Should return default address when RUTIFY_ADDR is not set
        assert!(addr.contains("127.0.0.1"));
        assert!(addr.contains("3000"));
    }

    #[test]
    fn test_resolve_service_addr_custom() {
        unsafe {
            std::env::set_var("RUTIFY_ADDR", "0.0.0.0:8080");
            let addr = resolve_service_addr();
            assert_eq!(addr, "http://127.0.0.1:8080");
            std::env::remove_var("RUTIFY_ADDR");
        }
    }

    #[test]
    fn test_notify_model_empty() {
        let items: Vec<CoreNotifyItem> = vec![];
        let model = notify_model(&items);
        assert_eq!(model.row_count(), 0);
    }

    #[test]
    fn test_notify_model_single_item() {
        let items = vec![CoreNotifyItem {
            id: 1,
            title: "Test".to_string(),
            notify: "Message".to_string(),
            device: "Device".to_string(),
            received_at: chrono::Utc::now(),
        }];

        let model = notify_model(&items);
        assert_eq!(model.row_count(), 1);
    }

    #[test]
    fn test_notify_model_multiple_items() {
        let items = vec![
            CoreNotifyItem {
                id: 1,
                title: "Test 1".to_string(),
                notify: "Message 1".to_string(),
                device: "Device 1".to_string(),
                received_at: chrono::Utc::now(),
            },
            CoreNotifyItem {
                id: 2,
                title: "Test 2".to_string(),
                notify: "Message 2".to_string(),
                device: "Device 2".to_string(),
                received_at: chrono::Utc::now(),
            },
        ];

        let model = notify_model(&items);
        assert_eq!(model.row_count(), 2);
    }

    #[test]
    fn test_apply_notifies_to_ui_empty() {
        let cache = Arc::new(std::sync::Mutex::new(Vec::<CoreNotifyItem>::new()));
        let items: Vec<CoreNotifyItem> = vec![];

        // This should not panic
        apply_notifies_to_ui(
            slint::Weak::default(), // Empty weak reference
            cache,
            items,
        );
    }

    #[test]
    fn test_apply_notifies_to_ui_with_items() {
        let cache = Arc::new(std::sync::Mutex::new(Vec::<CoreNotifyItem>::new()));
        let items = vec![CoreNotifyItem {
            id: 1,
            title: "Test".to_string(),
            notify: "Message".to_string(),
            device: "Device".to_string(),
            received_at: chrono::Utc::now(),
        }];

        // This should not panic
        apply_notifies_to_ui(
            slint::Weak::default(), // Empty weak reference
            cache.clone(),
            items,
        );

        // Verify the cache was updated
        let guard = cache.lock().unwrap();
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0].id, 1);
    }
}
