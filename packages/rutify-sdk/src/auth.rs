use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub jwt_token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub usage: String,
    pub expires_in_hours: Option<u64>,
    pub device_info: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub token_id: String,
    pub usage: String,
    pub token_type: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: i32,
    pub usage: String,
    pub token_type: String,
    pub device_info: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
}
