use clap::{Parser, Subcommand};
use tokio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use rutify::sdk::{RutifyClient, NotificationInput, NotificationMessage};

#[derive(Parser)]
#[command(name = "rutify-app")]
#[command(version = "1.0")]
#[command(about = "Rutify WebSocket Application")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:3000")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Listen for WebSocket notifications
    Listen,
    /// Send a notification and listen for response
    SendAndListen {
        #[arg(short, long, help = "Notification message")]
        message: String,
        #[arg(short, long, help = "Notification title")]
        title: Option<String>,
        #[arg(short, long, help = "Device name")]
        device: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = RutifyClient::new(&cli.server);
    let running = Arc::new(AtomicBool::new(true));

    match cli.command {
        Commands::Listen => {
            println!("🔌 Connecting to WebSocket at {}", cli.server);
            
            client.connect_websocket(move |msg| {
                match msg {
                    NotificationMessage::Event(event) => {
                        println!("📨 Received notification event:");
                        println!("   Event: {}", event.event);
                        println!("   Title: {}", event.data.title);
                        println!("   Message: {}", event.data.notify);
                        println!("   Device: {}", event.data.device);
                        println!("{}", "─".repeat(50));
                    }
                    NotificationMessage::Text(text) => {
                        println!("📄 Received text message: {}", text);
                    }
                    NotificationMessage::Close => {
                        println!("🔌 WebSocket connection closed");
                    }
                    NotificationMessage::Error(err) => {
                        println!("❌ WebSocket error: {}", err);
                    }
                }
            }).await?;

            println!("✅ Connected! Listening for notifications... Press Ctrl+C to stop.");

            // Keep the application running
            while running.load(Ordering::Relaxed) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            client.disconnect_websocket().await?;
            println!("👋 Disconnected");
        }
        Commands::SendAndListen { message, title, device } => {
            // First send notification
            let notification = NotificationInput {
                notify: message.clone(),
                title: title.clone(),
                device: device.clone(),
            };

            println!("📤 Sending notification: {}", message);
            client.send_notify(&notification).await?;
            println!("✅ Notification sent successfully!");

            // Then listen for responses
            println!("🔌 Connecting to WebSocket to listen for responses...");
            
            let message_clone = message.clone();
            client.connect_websocket(move |msg| {
                match msg {
                    NotificationMessage::Event(event) => {
                        println!("📨 Received event for our notification:");
                        println!("   Event: {}", event.event);
                        println!("   Title: {}", event.data.title);
                        println!("   Message: {}", event.data.notify);
                        println!("   Device: {}", event.data.device);
                        
                        // If this matches our sent notification, we can stop
                        if event.data.notify.contains(&message_clone) {
                            println!("✅ Received confirmation for our message!");
                        }
                        println!("{}", "─".repeat(50));
                    }
                    NotificationMessage::Text(text) => {
                        println!("📄 Received text: {}", text);
                    }
                    NotificationMessage::Close => {
                        println!("🔌 WebSocket connection closed");
                    }
                    NotificationMessage::Error(err) => {
                        println!("❌ WebSocket error: {}", err);
                    }
                }
            }).await?;

            println!("👂 Listening for responses for 10 seconds...");
            
            // Listen for 10 seconds then disconnect
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            
            client.disconnect_websocket().await?;
            println!("👋 Finished listening");
        }
    }

    Ok(())
}