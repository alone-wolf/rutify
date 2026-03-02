use anyhow::Result;
use clap::Subcommand;
use rutify_sdk::{CreateTokenRequest, LoginRequest, RegisterRequest, RutifyClient, TokenInfo};

#[derive(Subcommand)]
pub enum AuthAction {
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

pub async fn handle_auth_command(server: &str, action: AuthAction) -> Result<()> {
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
                        "💡 You can now login with: rutify-cli auth login --username {} --password <password>",
                        username
                    );
                }
                Err(e) => {
                    eprintln!("❌ Registration failed: {}", e);
                    std::process::exit(1);
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
                    std::process::exit(1);
                }
            }
        }

        AuthAction::Profile => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-cli auth login --username <user> --password <pass>");
                    std::process::exit(1);
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
                    std::process::exit(1);
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
                    eprintln!("💡 Please login first: rutify-cli auth login --username <user> --password <pass>");
                    std::process::exit(1);
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
                    std::process::exit(1);
                }
            }
        }

        AuthAction::ListTokens => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-cli auth login --username <user> --password <pass>");
                    std::process::exit(1);
                });

            let client = client.with_user_token(&user_token);

            println!("📋 Listing user tokens...");

            match client.get_user_tokens().await {
                Ok(tokens) => {
                    let tokens: Vec<TokenInfo> = tokens;
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
                    std::process::exit(1);
                }
            }
        }

        AuthAction::DeleteToken { id } => {
            let user_token = std::env::var("RUTIFY_USER_TOKEN")
                .unwrap_or_else(|_| {
                    eprintln!("❌ RUTIFY_USER_TOKEN environment variable not set");
                    eprintln!("💡 Please login first: rutify-cli auth login --username <user> --password <pass>");
                    std::process::exit(1);
                });

            let client = client.with_user_token(&user_token);

            println!("🗑️  Deleting token {}...", id);

            match client.delete_user_token(id).await {
                Ok(_) => {
                    println!("✅ Token {} deleted successfully!", id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to delete token: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
