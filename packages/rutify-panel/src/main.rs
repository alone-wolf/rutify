use clap::Parser;
use rutify_sdk::RutifyClient;
use std::sync::{Arc, Mutex};

mod tests;

#[derive(Parser)]
#[command(name = "rutify-panel")]
#[command(about = "Rutify Management Panel")]
pub struct Cli {
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub server: String,
}

slint::include_modules!();

struct ManagementState {
    client: RutifyClient,
    notifications: Arc<Mutex<Vec<rutify_sdk::NotifyItem>>>,
    stats: Arc<Mutex<Option<rutify_sdk::Stats>>>,
    tokens: Arc<Mutex<Vec<rutify_sdk::TokenItem>>>,
    devices: Arc<Mutex<Vec<rutify_sdk::DeviceInfo>>>,
}

impl ManagementState {
    fn new(server_url: &str) -> Self {
        Self {
            client: RutifyClient::new(server_url),
            notifications: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(None)),
            tokens: Arc::new(Mutex::new(Vec::new())),
            devices: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let state = ManagementState::new(&cli.server);

    run_management_panel(state).await?;
    Ok(())
}

async fn run_management_panel(state: ManagementState) -> anyhow::Result<()> {
    let ui = ManagementWindow::new()?;

    // Set up UI callbacks
    let notifications = Arc::clone(&state.notifications);
    let stats = Arc::clone(&state.stats);
    let tokens = Arc::clone(&state.tokens);
    let devices = Arc::clone(&state.devices);
    let client = state.client.clone();

    // Refresh data button
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let notifications_clone = Arc::clone(&notifications);
    let stats_clone = Arc::clone(&stats);
    let tokens_clone = Arc::clone(&tokens);
    let devices_clone = Arc::clone(&devices);

    ui.on_refresh_all(move || {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        let notifications = Arc::clone(&notifications_clone);
        let stats = Arc::clone(&stats_clone);
        let tokens = Arc::clone(&tokens_clone);
        let devices = Arc::clone(&devices_clone);

        tokio::spawn(async move {
            refresh_all_data(ui_weak, &client, &notifications, &stats, &tokens, &devices).await;
        });
    });

    // Delete notification
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let notifications_clone = Arc::clone(&notifications);

    ui.on_delete_notification(move |_id| {
        let ui_weak = ui_weak.clone();
        let _client = client_clone.clone();
        let _notifications = Arc::clone(&notifications_clone);

        tokio::spawn(async move {
            // This would be implemented when we have delete API
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status("Delete notification not yet implemented".into());
            }
        });
    });

    // Create token
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let tokens_clone = Arc::clone(&tokens);

    ui.on_create_token(move |_usage| {
        let ui_weak = ui_weak.clone();
        let _client = client_clone.clone();
        let _tokens = Arc::clone(&tokens_clone);

        tokio::spawn(async move {
            // This would be implemented when we have token management API
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status("Token creation not yet implemented".into());
            }
        });
    });

    // Delete token
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let tokens_clone = Arc::clone(&tokens);

    ui.on_delete_token(move |_id| {
        let ui_weak = ui_weak.clone();
        let _client = client_clone.clone();
        let _tokens = Arc::clone(&tokens_clone);

        tokio::spawn(async move {
            // This would be implemented when we have token management API
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status("Token deletion not yet implemented".into());
            }
        });
    });

    // Send test notification
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();

    ui.on_send_test_notification(move |message, title, device| {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();

        let input = rutify_sdk::NotificationInput {
            notify: message.to_string(),
            title: if title.is_empty() {
                None
            } else {
                Some(title.to_string())
            },
            device: if device.is_empty() {
                None
            } else {
                Some(device.to_string())
            },
        };

        tokio::spawn(async move {
            match client.send_notification(&input).await {
                Ok(_) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status("Test notification sent successfully!".into());
                    }
                }
                Err(e) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status(format!("Failed to send: {}", e).into());
                    }
                }
            }
        });
    });

    // Start WebSocket listener for real-time updates
    // 暂时禁用 WebSocket 监听器以避免 Send 问题
    // let ui_weak = ui.as_weak();
    // let client_clone = client.clone();
    // let notifications_clone = Arc::clone(&notifications);
    //
    // tokio::spawn(async move {
    //     if let Err(e) = start_websocket_listener(ui_weak, client_clone, notifications_clone).await {
    //         eprintln!("WebSocket listener errors: {}", e);
    //     }
    // });

    // Initial data load
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let notifications_clone = Arc::clone(&notifications);
    let stats_clone = Arc::clone(&stats);
    let tokens_clone = Arc::clone(&tokens);
    let devices_clone = Arc::clone(&devices);

    tokio::spawn(async move {
        refresh_all_data(
            ui_weak,
            &client_clone,
            &notifications_clone,
            &stats_clone,
            &tokens_clone,
            &devices_clone,
        )
        .await;
    });

    ui.run()?;
    Ok(())
}

async fn refresh_all_data(
    ui_weak: slint::Weak<ManagementWindow>,
    client: &RutifyClient,
    notifications: &Arc<Mutex<Vec<rutify_sdk::NotifyItem>>>,
    stats: &Arc<Mutex<Option<rutify_sdk::Stats>>>,
    _tokens: &Arc<Mutex<Vec<rutify_sdk::TokenItem>>>,
    _devices: &Arc<Mutex<Vec<rutify_sdk::DeviceInfo>>>,
) {
    // Load notifications
    match client.get_notifies().await {
        Ok(items) => {
            let mut guard = notifications.lock().unwrap();
            *guard = items;

            if let Some(ui) = ui_weak.upgrade() {
                update_notifications_ui(&ui, &guard);
            }
        }
        Err(e) => {
            eprintln!("Failed to load notifications: {}", e);
        }
    }

    // Load stats
    match client.get_stats().await {
        Ok(stats_data) => {
            let mut guard = stats.lock().unwrap();
            *guard = Some(stats_data);

            if let Some(ui) = ui_weak.upgrade() {
                update_stats_ui(&ui, &guard);
            }
        }
        Err(e) => {
            eprintln!("Failed to load stats: {}", e);
        }
    }

    // Tokens and devices would be loaded here when APIs are available
    if let Some(ui) = ui_weak.upgrade() {
        ui.set_status("Data refreshed".into());
    }
}

fn update_notifications_ui(ui: &ManagementWindow, notifications: &Vec<rutify_sdk::NotifyItem>) {
    // 简化版本，暂时不设置通知列表
    // TODO: 实现通知列表显示
    ui.set_status(format!("Loaded {} notifications", notifications.len()).into());
}

fn update_stats_ui(
    ui: &ManagementWindow,
    stats: &std::sync::MutexGuard<Option<rutify_sdk::Stats>>,
) {
    if let Some(stats_data) = stats.as_ref() {
        ui.set_today_count(stats_data.today_count);
        ui.set_total_count(stats_data.total_count);
        ui.set_device_count(stats_data.device_count);
        ui.set_server_status(
            if stats_data.is_running {
                "Running"
            } else {
                "Stopped"
            }
            .into(),
        );
        ui.set_uptime("Unknown".into()); // Would be calculated from server start time
    }
}

async fn start_websocket_listener(
    ui_weak: slint::Weak<ManagementWindow>,
    client: RutifyClient,
    notifications: Arc<Mutex<Vec<rutify_sdk::NotifyItem>>>,
) -> anyhow::Result<()> {
    match client.connect_websocket().await {
        Ok(mut rx) => {
            while let Some(msg) = rx.recv().await {
                match msg {
                    rutify_sdk::WebSocketMessage::Event(event) => {
                        // Add new notification to the list
                        let mut guard = notifications.lock().unwrap();
                        guard.insert(
                            0,
                            rutify_sdk::NotifyItem {
                                id: 0, // Will be set by server
                                title: event.data.title,
                                notify: event.data.notify,
                                device: event.data.device,
                                received_at: event.timestamp,
                            },
                        );

                        // Update UI
                        if let Some(ui) = ui_weak.upgrade() {
                            update_notifications_ui(&ui, &guard);

                            // Update stats
                            if let Ok(stats) = client.get_stats().await {
                                let stats_guard = Arc::new(Mutex::new(Some(stats)));
                                update_stats_ui(&ui, &stats_guard.lock().unwrap());
                            }
                        }
                    }
                    rutify_sdk::WebSocketMessage::Error { message } => {
                        eprintln!("WebSocket errors: {}", message);
                    }
                    rutify_sdk::WebSocketMessage::Close => {
                        println!("WebSocket connection closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect WebSocket: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
