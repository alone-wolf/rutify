use clap::{Parser, Subcommand};
use rutify_sdk::RutifyClient;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

#[derive(Parser)]
#[command(name = "rutify-application")]
#[command(about = "Rutify GUI application")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:3000")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the GUI application
    Gui,
    /// Listen for WebSocket notifications in console
    Listen,
    /// Send a notification and listen for response
    SendAndListen {
        /// Notification message
        #[arg(long)]
        message: String,
        /// Notification title
        #[arg(long)]
        title: Option<String>,
        /// Target device
        #[arg(long)]
        device: Option<String>,
    },
}

slint::include_modules!();

struct AppState {
    client: RutifyClient,
    notifications: Arc<Mutex<VecDeque<rutify_sdk::NotifyItem>>>,
    stats: Arc<Mutex<Option<rutify_sdk::Stats>>>,
}

impl AppState {
    fn new(server_url: &str) -> Self {
        Self {
            client: RutifyClient::new(server_url),
            notifications: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            stats: Arc::new(Mutex::new(None)),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let state = AppState::new(&cli.server);

    match cli.command {
        Commands::Gui => {
            run_gui(state).await?;
        }
        Commands::Listen => {
            listen_websocket(state).await?;
        }
        Commands::SendAndListen { message, title, device } => {
            send_and_listen(state, message, title, device).await?;
        }
    }

    Ok(())
}

async fn run_gui(state: AppState) -> anyhow::Result<()> {
    let ui = MainWindow::new()?;
    
    // Set up UI callbacks
    let notifications = Arc::clone(&state.notifications);
    let stats = Arc::clone(&state.stats);
    let client = state.client.clone();
    
    // Refresh button callback
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let notifications_clone = Arc::clone(&notifications);
    ui.on_refresh_clicked(move || {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        let notifications = Arc::clone(&notifications_clone);
        
        tokio::spawn(async move {
            match client.get_notifies().await {
                Ok(items) => {
                    let mut guard = notifications.lock().unwrap();
                    guard.clear();
                    guard.extend(items);
                    
                    if let Some(ui) = ui_weak.upgrade() {
                        update_ui_notifications(&ui, &guard);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to refresh notifications: {}", e);
                }
            }
        });
    });
    
    // Send notification callback
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    ui.on_send_notification(move |message, title, device| {
        let ui_weak = ui_weak.clone();
        let client = client_clone.clone();
        
        let input = rutify_sdk::NotificationInput {
            notify: message.to_string(),
            title: if title.is_empty() { None } else { Some(title.to_string()) },
            device: if device.is_empty() { None } else { Some(device.to_string()) },
        };
        
        tokio::spawn(async move {
            match client.send_notification(&input).await {
                Ok(_) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status("Notification sent successfully!".into());
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
    
    // Initial data load
    let ui_weak = ui.as_weak();
    let client_clone = client.clone();
    let notifications_clone = Arc::clone(&notifications);
    let stats_clone = Arc::clone(&stats);
    
    tokio::spawn(async move {
        // Load notifications
        match client_clone.get_notifies().await {
            Ok(items) => {
                let mut guard = notifications_clone.lock().unwrap();
                guard.clear();
                guard.extend(items);
                
                if let Some(ui) = ui_weak.upgrade() {
                    update_ui_notifications(&ui, &guard);
                }
            }
            Err(e) => {
                eprintln!("Failed to load notifications: {}", e);
            }
        }
        
        // Load stats
        match client_clone.get_stats().await {
            Ok(stats_data) => {
                let mut guard = stats_clone.lock().unwrap();
                *guard = Some(stats_data);
                
                if let Some(ui) = ui_weak.upgrade() {
                    update_ui_stats(&ui, &guard);
                }
            }
            Err(e) => {
                eprintln!("Failed to load stats: {}", e);
            }
        }
    });
    
    ui.run()?;
    Ok(())
}

fn update_ui_notifications(ui: &MainWindow, notifications: &VecDeque<rutify_sdk::NotifyItem>) {
    // 简化版本，暂时不设置通知列表
    // TODO: 实现通知列表显示
    ui.set_status(format!("Loaded {} notifications", notifications.len()).into());
}

fn update_ui_stats(ui: &MainWindow, stats: &std::sync::MutexGuard<Option<rutify_sdk::Stats>>) {
    if let Some(stats_data) = stats.as_ref() {
        ui.set_today_count(stats_data.today_count);
        ui.set_total_count(stats_data.total_count);
        ui.set_device_count(stats_data.device_count);
        ui.set_server_status(if stats_data.is_running { "Running" } else { "Stopped" }.into());
    }
}

async fn listen_websocket(state: AppState) -> anyhow::Result<()> {
    println!("🎧 Listening for WebSocket notifications...");
    println!("   Press Ctrl+C to stop");
    
    match state.client.connect_websocket().await {
        Ok(mut rx) => {
            while let Some(msg) = rx.recv().await {
                match msg {
                    rutify_sdk::WebSocketMessage::Event(event) => {
                        println!("🔔 New notification:");
                        println!("   Title: {}", event.data.title);
                        println!("   Message: {}", event.data.notify);
                        println!("   Device: {}", event.data.device);
                        println!("   Time: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S"));
                        println!();
                        
                        // Add to local cache
                        let mut guard = state.notifications.lock().unwrap();
                        if guard.len() >= 100 {
                            guard.pop_front();
                        }
                        guard.push_back(rutify_sdk::NotifyItem {
                            id: 0, // Will be set by server
                            title: event.data.title,
                            notify: event.data.notify,
                            device: event.data.device,
                            received_at: event.timestamp,
                        });
                    }
                    rutify_sdk::WebSocketMessage::Text(text) => {
                        println!("📝 Text message: {}", text);
                    }
                    rutify_sdk::WebSocketMessage::Error { message } => {
                        eprintln!("❌ Error: {}", message);
                    }
                    rutify_sdk::WebSocketMessage::Close => {
                        println!("🔌 Connection closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to connect WebSocket: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn send_and_listen(
    state: AppState,
    message: String,
    title: Option<String>,
    device: Option<String>,
) -> anyhow::Result<()> {
    let input = rutify_sdk::NotificationInput {
        notify: message,
        title,
        device,
    };
    
    println!("📤 Sending notification and listening for response...");
    
    match state.client.connect_websocket().await {
        Ok(mut rx) => {
            // Send notification
            state.client.send_notification(&input).await?;
            println!("✅ Notification sent, waiting for response...");
            
            // Listen for response
            while let Some(msg) = rx.recv().await {
                match msg {
                    rutify_sdk::WebSocketMessage::Event(event) => {
                        println!("🔔 Response received:");
                        println!("   Title: {}", event.data.title);
                        println!("   Message: {}", event.data.notify);
                        println!("   Device: {}", event.data.device);
                        println!("   Time: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S"));
                        break;
                    }
                    rutify_sdk::WebSocketMessage::Text(text) => {
                        println!("📝 Response: {}", text);
                        break;
                    }
                    rutify_sdk::WebSocketMessage::Error { message } => {
                        eprintln!("❌ Error: {}", message);
                        break;
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to connect WebSocket: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
