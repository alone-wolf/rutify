use clap::{Parser, Subcommand};
use rutify_sdk::RutifyClient;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = RutifyClient::new(&cli.server);

    match cli.command {
        Commands::Notifies => {
            match client.get_notifies().await {
                Ok(notifies) => {
                    println!("📬 Notifications ({} total):", notifies.len());
                    for (i, notify) in notifies.iter().enumerate() {
                        println!("  {}. {} - {} ({})", 
                            i + 1, 
                            notify.title, 
                            notify.notify, 
                            notify.device
                        );
                        println!("     Received: {}", notify.received_at.format("%Y-%m-%d %H:%M:%S"));
                        if i < notifies.len() - 1 {
                            println!();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to get notifies: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Stats => {
            match client.get_stats().await {
                Ok(stats) => {
                    println!("📊 Server Statistics:");
                    println!("  Today's notifications: {}", stats.today_count);
                    println!("  Total notifications: {}", stats.total_count);
                    println!("  Active devices: {}", stats.device_count);
                    println!("  Server running: {}", if stats.is_running { "✅ Yes" } else { "❌ No" });
                }
                Err(e) => {
                    eprintln!("❌ Failed to get stats: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Send { message, title, device } => {
            let input = rutify_sdk::NotificationInput {
                notify: message,
                title,
                device,
            };
            
            match client.send_notification(&input).await {
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
            
            match client.connect_websocket().await {
                Ok(mut rx) => {
                    while let Some(msg) = rx.recv().await {
                        match msg {
                            rutify_sdk::WebSocketMessage::Event(event) => {
                                println!("🔔 New notification:");
                                println!("   Title: {}", event.data.title);
                                println!("   Message: {}", event.data.notify);
                                println!("   Device: {}", event.data.device);
                                println!("   Time: {}", event.timestamp.format("%Y-%m-%d %H:%M:%S"));
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
                    std::process::exit(1);
                }
            }
        }
        Commands::SendAndListen { message, title, device } => {
            let input = rutify_sdk::NotificationInput {
                notify: message,
                title,
                device,
            };
            
            println!("📤 Sending notification and listening for response...");
            
            match client.connect_websocket().await {
                Ok(mut rx) => {
                    // Send notification
                    if let Err(e) = client.send_notification(&input).await {
                        eprintln!("❌ Failed to send notification: {}", e);
                        std::process::exit(1);
                    }
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
                    std::process::exit(1);
                }
            }
        }
        Commands::Devices => {
            // This would be implemented when we have device management API
            println!("📱 Device management not yet implemented");
        }
        Commands::Health => {
            // Simple health check by trying to get stats
            match client.get_stats().await {
                Ok(_) => {
                    println!("✅ Server is healthy and responsive");
                }
                Err(e) => {
                    eprintln!("❌ Server health check failed: {}", e);
                    std::process::exit(1);
                }
            }
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
        let args = vec!["rutify-cli", "--server", "http://localhost:8080", "notifies"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        assert_eq!(cli.server, "http://localhost:8080");
        match cli.command {
            Commands::Notifies => {}, // Expected
            _ => panic!("Expected Notifies command"),
        }
    }

    #[test]
    fn test_cli_default_server() {
        let args = vec!["rutify-cli", "stats"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        assert_eq!(cli.server, "http://127.0.0.1:3000");
        match cli.command {
            Commands::Stats => {}, // Expected
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_send_command_parsing() {
        let args = vec![
            "rutify-cli",
            "send",
            "Hello World",
            "--title", "Test Title",
            "--device", "test-device"
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command {
            Commands::Send { message, title, device } => {
                assert_eq!(message, "Hello World");
                assert_eq!(title, Some("Test Title".to_string()));
                assert_eq!(device, Some("test-device".to_string()));
            },
            _ => panic!("Expected Send command"),
        }
    }

    #[test]
    fn test_send_command_optional_fields() {
        let args = vec!["rutify-cli", "send", "Hello World"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command {
            Commands::Send { message, title, device } => {
                assert_eq!(message, "Hello World");
                assert_eq!(title, None);
                assert_eq!(device, None);
            },
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
            "--title", "测试标题",
            "--device", "デバイス"
        ];
        
        let result = Cli::try_parse_from(args);
        assert!(result.is_ok());
        
        if let Ok(cli) = result {
            match cli.command {
                Commands::Send { message, title, device } => {
                    assert_eq!(message, "🚀 Hello World 🌍");
                    assert_eq!(title.unwrap(), "测试标题");
                    assert_eq!(device.unwrap(), "デバイス");
                },
                _ => panic!("Expected Send command"),
            }
        }
    }
}
