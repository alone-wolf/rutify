use clap::{Parser, Subcommand};
use rutify_client::{
    ClientState, WebSocketNotification, format_notification, format_stats, health_check,
    send_and_listen,
};

mod auth_commands;
mod token_commands;

#[derive(Parser)]
#[command(name = "rutify-cli")]
#[command(about = "Rutify CLI client")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:3000")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get all notifications
    Notifies,
    /// Get server statistics
    Stats,
    /// Send a notification
    Send {
        /// Notification message
        message: String,
        /// Notification title
        #[arg(long)]
        title: Option<String>,
        /// Target device
        #[arg(long)]
        device: Option<String>,
    },
    /// Listen for WebSocket notifications
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
    /// List available devices
    Devices,
    /// Server health check
    Health,
    /// Token management
    Token {
        #[command(subcommand)]
        action: token_commands::TokenAction,
    },
    /// User authentication
    Auth {
        #[command(subcommand)]
        action: auth_commands::AuthAction,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut state = ClientState::new(&cli.server);

    match cli.command {
        Commands::Notifies => match state.get_notifies().await {
            Ok(notifies) => {
                println!("📬 Notifications ({} total):", notifies.len());
                for (i, notify) in notifies.iter().enumerate() {
                    println!("  {}. {}", i + 1, format_notification(notify));
                    if i < notifies.len() - 1 {
                        println!();
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to get notifies: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Stats => match state.get_stats().await {
            Ok(stats) => {
                println!("📊 Server Statistics:");
                println!("  {}", format_stats(&stats));
            }
            Err(e) => {
                eprintln!("❌ Failed to get stats: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Send {
            message,
            title,
            device,
        } => {
            let input = rutify_sdk::NotificationInput {
                notify: message,
                title,
                device,
            };

            match state.send_notification(&input).await {
                Ok(_) => {
                    println!("✅ Notification sent successfully!");
                }
                Err(e) => {
                    eprintln!("❌ Failed to send notification: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Listen => {
            println!("🎧 Listening for WebSocket notifications...");
            println!("   Press Ctrl+C to stop");

            match state.listen_websocket_updates().await {
                Ok(mut rx) => {
                    while let Some(notification) = rx.recv().await {
                        match notification {
                            WebSocketNotification::Event(event) => {
                                println!("🔔 New notification:");
                                println!("   Title: {}", event.data.title);
                                println!("   Message: {}", event.data.notify);
                                println!("   Device: {}", event.data.device);
                                println!(
                                    "   Time: {}",
                                    event.timestamp.format("%Y-%m-%d %H:%M:%S")
                                );
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
                    std::process::exit(1);
                }
            }
        }
        Commands::SendAndListen {
            message,
            title,
            device,
        } => {
            println!("📤 Sending notification and listening for response...");

            match send_and_listen(&state, message, title, device).await {
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
                    std::process::exit(1);
                }
            }
        }
        Commands::Devices => {
            // This would be implemented when we have device management API
            println!("📱 Device management not yet implemented");
        }
        Commands::Health => match health_check(&state).await {
            Ok(true) => {
                println!("✅ Server is healthy and responsive");
            }
            Ok(false) => {
                eprintln!("❌ Server health check failed");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("❌ Server health check failed: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Token { action } => {
            token_commands::handle_token_command(&mut state, action).await?;
        }
        Commands::Auth { action } => {
            auth_commands::handle_auth_command(&cli.server, action).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        let args = vec![
            "rutify-cli",
            "--server",
            "http://localhost:8080",
            "notifies",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.server, "http://localhost:8080");
        match cli.command {
            Commands::Notifies => {} // Expected
            _ => panic!("Expected Notifies command"),
        }
    }

    #[test]
    fn test_cli_default_server() {
        let args = vec!["rutify-cli", "stats"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.server, "http://127.0.0.1:3000");
        match cli.command {
            Commands::Stats => {} // Expected
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_send_command_parsing() {
        let args = vec![
            "rutify-cli",
            "send",
            "Hello World",
            "--title",
            "Test Title",
            "--device",
            "test-device",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Send {
                message,
                title,
                device,
            } => {
                assert_eq!(message, "Hello World");
                assert_eq!(title, Some("Test Title".to_string()));
                assert_eq!(device, Some("test-device".to_string()));
            }
            _ => panic!("Expected Send command"),
        }
    }

    #[test]
    fn test_send_command_optional_fields() {
        let args = vec!["rutify-cli", "send", "Hello World"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Send {
                message,
                title,
                device,
            } => {
                assert_eq!(message, "Hello World");
                assert_eq!(title, None);
                assert_eq!(device, None);
            }
            _ => panic!("Expected Send command"),
        }
    }

    #[test]
    fn test_all_commands_exist() {
        let commands = vec![
            vec!["rutify-cli", "notifies"],
            vec!["rutify-cli", "stats"],
            vec!["rutify-cli", "send", "test"],
            vec!["rutify-cli", "listen"],
            vec!["rutify-cli", "send-and-listen", "--message", "test"],
            vec!["rutify-cli", "devices"],
            vec!["rutify-cli", "health"],
        ];

        for args in commands {
            let result = Cli::try_parse_from(args.clone());
            assert!(result.is_ok(), "Failed to parse: {:?}", args);
        }
    }

    #[test]
    fn test_unicode_arguments() {
        let args = vec![
            "rutify-cli",
            "send",
            "🚀 Hello World 🌍",
            "--title",
            "测试标题",
            "--device",
            "デバイス",
        ];

        let result = Cli::try_parse_from(args);
        assert!(result.is_ok());

        if let Ok(cli) = result {
            match cli.command {
                Commands::Send {
                    message,
                    title,
                    device,
                } => {
                    assert_eq!(message, "🚀 Hello World 🌍");
                    assert_eq!(title.unwrap(), "测试标题");
                    assert_eq!(device.unwrap(), "デバイス");
                }
                _ => panic!("Expected Send command"),
            }
        }
    }
}
