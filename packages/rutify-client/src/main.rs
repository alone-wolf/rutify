use clap::{Parser, Subcommand};
use tokio;
use rutify_sdk::{RutifyClient, NotificationInput};

#[derive(Parser)]
#[command(name = "rutify-client")]
#[command(version = "1.0")]
#[command(about = "Rutify SDK Client")]
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

    match cli.command {
        Commands::Notifies => {
            match client.get_notifies().await {
                Ok(notifies) => {
                    println!("📬 Notifications ({} total):", notifies.len());
                    println!("{}", "─".repeat(60));
                    for notify in notifies {
                        println!("🔔 ID: {}", notify.id);
                        println!("   Title: {}", notify.title);
                        println!("   Message: {}", notify.notify);
                        println!("   Device: {}", notify.device);
                        println!("   Time: {}", notify.received_at);
                        println!("{}", "─".repeat(60));
                    }
                }
                Err(e) => eprintln!("❌ Failed to get notifies: {}", e),
            }
        }
        Commands::Stats => {
            match client.get_stats().await {
                Ok(stats) => {
                    println!("📊 Server Statistics:");
                    println!("{}", "─".repeat(30));
                    println!("📈 Today's count: {}", stats.today_count);
                    println!("📈 Total count: {}", stats.total_count);
                    println!("📱 Device count: {}", stats.device_count);
                    println!("🟢 Status: {}", if stats.is_running { "Running" } else { "Stopped" });
                }
                Err(e) => eprintln!("❌ Failed to get stats: {}", e),
            }
        }
        Commands::Send { message, title, device } => {
            let notification = NotificationInput {
                notify: message.clone(),
                title,
                device,
            };

            match client.send_notify(&notification).await {
                Ok(_) => println!("✅ Notification sent successfully: {}", message),
                Err(e) => eprintln!("❌ Failed to send notification: {}", e),
            }
        }
    }

    Ok(())
}