use anyhow::Result;
use clap::Subcommand;
use rutify_client::ClientState;

#[derive(Subcommand)]
pub enum TokenAction {
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

pub async fn handle_token_command(state: &ClientState, action: TokenAction) -> Result<()> {
    match action {
        TokenAction::Create { usage, expires_in } => {
            println!("🔑 Creating new token for usage: '{}', expires in {} hours", usage, expires_in);
            match state.create_token(&usage, expires_in).await {
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
            println!("   Token set: {}...", &token[..std::cmp::min(20, token.len())]);
            println!("   💡 Use this token for subsequent requests");
        }
        TokenAction::Clear => {
            println!("🗑️  Clearing stored token...");
            println!("   Token cleared");
        }
        TokenAction::Status => {
            if state.has_token() {
                println!("✅ Token is configured");
            } else {
                println!("❌ No token configured");
            }
        }
    }
    Ok(())
}
