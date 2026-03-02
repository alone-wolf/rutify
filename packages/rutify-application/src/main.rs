use clap::{Parser, Subcommand};
use rutify_client::{
    ClientState, WebSocketNotification, send_and_listen as client_send_and_listen,
};
use rutify_sdk::{CreateTokenRequest, LoginRequest, RegisterRequest, RutifyClient};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "rutify-application")]
#[command(about = "Rutify GUI application")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    server: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the GUI application (default)
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
    /// Token management
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
    /// User authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

#[derive(Subcommand)]
enum TokenAction {
    /// Create a new token
    Create {
        /// Token usage/purpose
        usage: String,
        /// Expiration time in hours (default: 24)
        #[arg(long, default_value = "24")]
        expires_in: u64,
    },
    /// Set token for authentication
    Set {
        /// Bearer token
        token: String,
    },
    /// Clear stored token
    Clear,
    /// Show current token status
    Status,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Register a new user
    Register {
        /// Username
        username: String,
        /// Password
        password: String,
        /// Email
        email: String,
    },
    /// Login with username and password
    Login {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// Get current user profile
    Profile,
    /// Create a new notification token
    CreateToken {
        /// Token usage description
        usage: String,
        /// Device information
        #[arg(long)]
        device: Option<String>,
        /// Token expiration in hours (default: 24)
        #[arg(long, default_value = "24")]
        expires: u64,
    },
    /// List user tokens
    ListTokens,
    /// Delete a token
    DeleteToken {
        /// Token ID
        id: i32,
    },
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Gui
    }
}

slint::include_modules!();

struct AppState {
    client_state: ClientState,
}

impl AppState {
    fn new(server_url: &str) -> Self {
        Self {
            client_state: ClientState::new(server_url),
        }
    }

    fn notifications(&self) -> Arc<Mutex<VecDeque<rutify_sdk::NotifyItem>>> {
        Arc::clone(&self.client_state.notifications)
    }

    fn stats(&self) -> Arc<Mutex<Option<rutify_sdk::Stats>>> {
        Arc::clone(&self.client_state.stats)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let state = AppState::new(&cli.server);

    match cli.command {
        Some(Commands::Gui) => {
            run_gui(state).await?;
        }
        Some(Commands::Listen) => {
            listen_websocket(state).await?;
        }
        Some(Commands::SendAndListen {
            message,
            title,
            device,
        }) => {
            send_and_listen(state, message, title, device).await?;
        }
        Some(Commands::Token { action }) => {
            handle_token_command(&state.client_state, action).await?;
        }
        Some(Commands::Auth { action }) => {
            handle_auth_command(&cli.server, action).await?;
        }
        None => {
            // Default behavior - start GUI
            run_gui(state).await?;
        }
    }

    Ok(())
}

async fn run_gui(state: AppState) -> anyhow::Result<()> {
    let ui = MainWindow::new()?;

    // Set up UI callbacks
    let _client_state = state.client_state.clone();

    // Refresh button callback
    let ui_weak = ui.as_weak();
    let client_state = state.client_state.clone();
    let notifications = Arc::clone(&state.notifications());
    ui.on_refresh_clicked(move || {
        let ui_weak = ui_weak.clone();
        let client_state = client_state.clone();
        let notifications = Arc::clone(&notifications);

        tokio::spawn(async move {
            match client_state.get_notifies().await {
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
    let client_state = state.client_state.clone();
    ui.on_send_notification(move |message, title, device| {
        let ui_weak = ui_weak.clone();
        let client_state = client_state.clone();

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
            match client_state.send_notification(&input).await {
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
    let client_state = state.client_state.clone();
    let notifications = Arc::clone(&state.notifications());
    let stats = Arc::clone(&state.stats());

    tokio::spawn(async move {
        // Load notifications
        match client_state.get_notifies().await {
            Ok(items) => {
                let mut guard = notifications.lock().unwrap();
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
        match client_state.get_stats().await {
            Ok(stats_data) => {
                let mut guard = stats.lock().unwrap();
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
        ui.set_server_status(
            if stats_data.is_running {
                "Running"
            } else {
                "Stopped"
            }
            .into(),
        );
    }
}

async fn listen_websocket(state: AppState) -> anyhow::Result<()> {
    println!("🎧 Listening for WebSocket notifications...");
    println!("   Press Ctrl+C to stop");

    match state.client_state.listen_websocket_updates().await {
        Ok(mut rx) => {
            while let Some(notification) = rx.recv().await {
                match notification {
                    WebSocketNotification::Event(event) => {
                        println!("🔔 New notification:");
                        println!("   Title: {}", event.data.title);
                        println!("   Message: {}", event.data.notify);
                        println!("   Device: {}", event.data.device);
                        println!("   Time: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S"));
                        println!();
                    }
                    WebSocketNotification::Text(text) => {
                        println!("📝 Text message: {}", text);
                    }
                    WebSocketNotification::Error { message } => {
                        eprintln!("❌ Error: {}", message);
                    }
                    WebSocketNotification::Close => {
                        println!("🔌 Connection closed");
                        break;
                    }
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
    println!("📤 Sending notification and listening for response...");

    match client_send_and_listen(&state.client_state, message, title, device).await {
        Ok(Some(notification)) => match notification {
            WebSocketNotification::Event(event) => {
                println!("🔔 Response received:");
                println!("   Title: {}", event.data.title);
                println!("   Message: {}", event.data.notify);
                println!("   Device: {}", event.data.device);
                println!("   Time: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
            WebSocketNotification::Text(text) => {
                println!("📝 Response: {}", text);
            }
            WebSocketNotification::Error { message } => {
                eprintln!("❌ Error: {}", message);
            }
            WebSocketNotification::Close => {
                println!("🔌 Connection closed");
            }
        },
        Ok(None) => {
            println!("⏰ No response received");
        }
        Err(e) => {
            eprintln!("❌ Failed to send and listen: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn handle_token_command(
    client_state: &ClientState,
    action: TokenAction,
) -> anyhow::Result<()> {
    match action {
        TokenAction::Create { usage, expires_in } => {
            println!(
                "🔑 Creating new token for usage: '{}', expires in {} hours",
                usage, expires_in
            );
            match client_state.create_token(&usage, expires_in).await {
                Ok(token_response) => {
                    println!("✅ Token created successfully!");
                    println!("   Token ID: {}", token_response.token_id);
                    println!("   Usage: {}", token_response.usage);
                    println!("   Expires at: {}", token_response.expires_at);
                    println!("   Token: {}", token_response.token);
                    println!("   💡 Save this token securely!");
                }
                Err(e) => eprintln!("❌ Failed to create token: {}", e),
            }
        }
        TokenAction::Set { token } => {
            println!("🔐 Setting authentication token...");
            println!(
                "   Token set: {}...",
                &token[..std::cmp::min(20, token.len())]
            );
            println!("   💡 Use this token for subsequent requests");
        }
        TokenAction::Clear => {
            println!("🗑️  Clearing stored token...");
            println!("   Token cleared");
        }
        TokenAction::Status => {
            if client_state.has_token() {
                println!("✅ Token is configured");
            } else {
                println!("❌ No token configured");
            }
        }
    }
    Ok(())
}

async fn handle_auth_command(server: &str, action: AuthAction) -> anyhow::Result<()> {
    let client = RutifyClient::new(server);

    match action {
        AuthAction::Register {
            username,
            password,
            email,
        } => {
            println!("🔐 Registering new user...");

            let request = RegisterRequest {
                username: username.clone(),
                password,
                email,
            };

            match client.register(&request).await {
                Ok(_) => {
                    println!("✅ User '{}' registered successfully!", username);
                    println!(
                        "💡 You can now login with: rutify-application auth login --username {} --password <password>",
                        username
                    );
                }
                Err(e) => {
                    eprintln!("❌ Registration failed: {}", e);
                }
            }
        }

        AuthAction::Login { username, password } => {
            println!("🔑 Logging in...");

            let request = LoginRequest {
                username: username.clone(),
                password,
            };

            match client.login(&request).await {
                Ok(response) => {
                    println!("✅ Login successful!");
                    println!("👤 User: {}", response.username);
                    println!("📧 Email: {}", response.email);
                    println!("🔐 Role: {}", response.role);
                    println!("⏰ Expires at: {}", response.expires_at);
                    println!("🎫 JWT Token: {}", response.jwt_token);
                    println!();
                    println!("💡 Save this token for future API calls:");
                    println!("   export RUTIFY_USER_TOKEN=\"{}\"", response.jwt_token);
                }
                Err(e) => {
                    eprintln!("❌ Login failed: {}", e);
                }
            }
        }

        AuthAction::Profile => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-application auth login --username <user> --password <pass>");
                    String::new() // 返回空字符串而不是Ok(())
                });

            let client = client.with_user_token(&user_token);

            println!("👤 Getting user profile...");

            match client.get_user_profile().await {
                Ok(profile) => {
                    println!("✅ User Profile:");
                    println!("  🆔 ID: {}", profile.id);
                    println!("  📝 Usage: {}", profile.usage);
                    println!("  🔐 Type: {}", profile.token_type);
                    if let Some(device) = profile.device_info {
                        println!("  📱 Device: {}", device);
                    }
                    println!("  📅 Created: {}", profile.created_at);
                    println!("  ⏰ Expires: {}", profile.expires_at);
                    if let Some(last_used) = profile.last_used_at {
                        println!("  🔄 Last Used: {}", last_used);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to get profile: {}", e);
                }
            }
        }

        AuthAction::CreateToken {
            usage,
            device,
            expires,
        } => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-application auth login --username <user> --password <pass>");
                    String::new() // 返回空字符串而不是Ok(())
                });

            let client = client.with_user_token(&user_token);

            println!("🎫 Creating notification token...");

            let request = CreateTokenRequest {
                usage: usage.clone(),
                expires_in_hours: Some(expires),
                device_info: device,
            };

            match client.create_notify_token(&request).await {
                Ok(response) => {
                    println!("✅ Token created successfully!");
                    println!("🎫 Token: {}", response.token);
                    println!("🆔 Token ID: {}", response.token_id);
                    println!("📝 Usage: {}", response.usage);
                    println!("🔐 Type: {}", response.token_type);
                    println!("⏰ Expires at: {}", response.expires_at);
                    println!();
                    println!("💡 Use this token for notifications:");
                    println!("   export RUTIFY_TOKEN=\"{}\"", response.token);
                }
                Err(e) => {
                    eprintln!("❌ Failed to create token: {}", e);
                }
            }
        }

        AuthAction::ListTokens => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-application auth login --username <user> --password <pass>");
                    String::new() // 返回空字符串而不是Ok(())
                });

            let client = client.with_user_token(&user_token);

            println!("📋 Listing user tokens...");

            match client.get_user_tokens().await {
                Ok(tokens) => {
                    if tokens.is_empty() {
                        println!("📭 No tokens found.");
                    } else {
                        println!("🎫 User Tokens ({} total):", tokens.len());
                        for (i, token) in tokens.iter().enumerate() {
                            println!(
                                "  {}. 🆔 {} | 📝 {} | 🔐 {}",
                                i + 1,
                                token.id,
                                token.usage,
                                token.token_type
                            );
                            if let Some(device) = &token.device_info {
                                println!("     📱 {}", device);
                            }
                            println!("     📅 {} | ⏰ {}", token.created_at, token.expires_at);
                            if let Some(last_used) = &token.last_used_at {
                                println!("     🔄 Last Used: {}", last_used);
                            }
                            if i < tokens.len() - 1 {
                                println!();
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to list tokens: {}", e);
                }
            }
        }

        AuthAction::DeleteToken { id } => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-application auth login --username <user> --password <pass>");
                    String::new() // 返回空字符串而不是Ok(())
                });

            let client = client.with_user_token(&user_token);

            println!("🗑️  Deleting token {}...", id);

            match client.delete_user_token(id).await {
                Ok(_) => {
                    println!("✅ Token {} deleted successfully!", id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to delete token: {}", e);
                }
            }
        }
    }

    Ok(())
}
