mod app;
mod db;
mod error;
mod routes;
mod state;

slint::include_modules!();

use rutify_sdk::{RutifyClient, NotifyItemData};
use crate::state::AppState;
use clap::Parser;
use dotenvy::dotenv;
use sea_orm::{Database, DbErr};
use slint::{ModelRc, VecModel};
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(clap::Parser)]
struct CliArgs {
    #[clap(long)]
    ui: bool,
}

fn main() -> anyhow::Result<()> {
    // let ui = AppWindow::new().unwrap().run().unwrap();
    // ui.show();

    let args = CliArgs::parse();
    println!("ui:{}", args.ui);

    // init_logging(&args);

    match args.ui {
        true => run_with_ui()?,
        false => run_cli_only()?,
    }

    Ok(())
}

fn run_cli_only() -> anyhow::Result<()> {
    dotenv().ok();
    
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rutify=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let rt = tokio::runtime::Runtime::new()?;
    let _r = rt.block_on(async { rutify_service().await });

    Ok(())
}

fn run_with_ui() -> anyhow::Result<()> {
    dotenv().ok();
    
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rutify=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ui = AppWindow::new()?;
    let rt = tokio::runtime::Runtime::new()?;
    let rt_handle = rt.handle().clone();
    let weak_ui = ui.as_weak();
    let service_addr = resolve_service_addr();
    let sdk_client = RutifyClient::new(&service_addr);
    let cached_notifies: Arc<Mutex<Vec<NotifyItemData>>> = Arc::new(Mutex::new(Vec::new()));
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

fn notify_model(items: &[NotifyItemData]) -> ModelRc<NotifyItem> {
    let converted: Vec<NotifyItem> = items
        .iter()
        .map(|item| NotifyItem {
            id: item.id,
            title: item.title.clone().into(),
            notify: item.notify.clone().into(),
            device: item.device.clone().into(),
            received_at: item.received_at.clone().into(),
        })
        .collect();
    ModelRc::new(VecModel::from(converted))
}

fn apply_notifies_to_ui(
    ui: slint::Weak<AppWindow>,
    cache: Arc<Mutex<Vec<NotifyItemData>>>,
    items: Vec<NotifyItemData>,
) {
    {
        let mut guard = cache.lock().unwrap();
        *guard = items.clone();
    }

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui.upgrade() {
            let recent: Vec<NotifyItemData> = items.iter().take(5).cloned().collect();
            ui.set_all_notifies(notify_model(&items));
            ui.set_recent_notifies(notify_model(&recent));
        }
    });
}

async fn rutify_service() -> Result<(), DbErr> {
    let db_url = std::env::var("RUTIFY_DB_URL")
        .unwrap_or_else(|_| "sqlite://rutify.db?mode=rwc".to_string());
    let db_cnn = Database::connect(&db_url).await?;
    db::initialize::initial(&db_cnn).await;

    let (tx, _) = broadcast::channel(200);
    let state = Arc::new(AppState { db: db_cnn, tx });

    let app = app::axum_app(state);

    let addr: SocketAddr = std::env::var("RUTIFY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("invalid RUTIFY_ADDR");

    info!(
        "rutify started at {}://{}",
        "http",
        addr.clone().to_string().replace("0.0.0.0", "127.0.0.1")
    );
    let tcp = TcpListener::bind(addr).await.unwrap();
    axum::serve(tcp, app).await.unwrap();
    Ok(())
}
