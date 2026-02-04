use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::token_ops;
use crate::state::AppState;
use crate::error::AppError;

/// JWT Claims 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Token ID
    pub usage: String,      // Token用途
    pub iat: i64,          // 签发时间
    pub exp: i64,          // 过期时间
    pub jti: String,       // JWT ID
}

/// Token 创建请求
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub usage: String,
    pub expires_in_hours: Option<u64>,
}

/// Token 创建响应
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub token_id: String,
    pub usage: String,
    pub expires_at: String,
}

/// Bearer Token 提取器
pub struct BearerToken(pub String);

/// JWT 密钥 (从环境变量获取，默认使用固定密钥)
fn get_jwt_secret() -> String {
    let secret = std::env::var("RUTIFY_JWT_SECRET").unwrap_or_else(|_| {
        warn!("Using default JWT secret. Please set RUTIFY_JWT_SECRET environment variable in production!");
        "rutify_default_jwt_secret_change_in_production".to_string()
    });
    
    // 验证密钥强度
    if secret.len() < 32 {
        error!("JWT secret is too short (minimum 32 characters required)");
        panic!("JWT secret must be at least 32 characters long");
    }
    
    secret
}

/// 生成 Token Hash
pub fn generate_token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// 创建新的 JWT Token
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, AppError> {
    let token_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let expires_in = request.expires_in_hours.unwrap_or(24); // 默认24小时
    let expires_at = now + chrono::Duration::hours(expires_in as i64);
    
    let claims = Claims {
        sub: token_id.clone(),
        usage: request.usage.clone(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };

    let secret = get_jwt_secret();
    
    // 明确指定HS256算法
    let header = Header::new(jsonwebtoken::Algorithm::HS256);
    
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| {
        error!("Failed to encode JWT: {}", e);
        AppError::AuthError("Failed to create token".to_string())
    })?;

    // 保存 token hash 到数据库
    let token_hash = generate_token_hash(&token);
    token_ops::create_token(&state.db, &token_hash, &request.usage, expires_at).await?;

    info!("Created new token for usage: {}", request.usage);

    Ok(Json(CreateTokenResponse {
        token,
        token_id,
        usage: request.usage,
        expires_at: expires_at.to_string(),
    }))
}

/// 验证 JWT Token
pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let secret = get_jwt_secret();
    
    // 创建严格的验证配置
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true; // 验证过期时间
    validation.leeway = 60; // 允许60秒的时钟偏差
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )
    .map_err(|e| {
        error!("JWT verification failed: {}", e);
        AppError::AuthError("Invalid token".to_string())
    })?;

    Ok(token_data.claims)
}

/// 从请求头中提取 Bearer Token
pub fn extract_bearer_token(request: &Request) -> Result<BearerToken, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| AppError::AuthError("Missing authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::AuthError("Invalid authorization header format".to_string()));
    }

    let token = auth_header.trim_start_matches("Bearer ").to_string();
    Ok(BearerToken(token))
}

/// 授权中间件
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let BearerToken(token) = extract_bearer_token(&request)?;
    
    // 验证 JWT
    let claims = verify_token(&token)?;
    
    // 验证 token 是否在数据库中存在且未过期
    let token_hash = generate_token_hash(&token);
    if !token_ops::verify_token_exists(&state.db, &token_hash).await? {
        return Err(AppError::AuthError("Token not found or expired".to_string()));
    }

    // 将 claims 添加到请求扩展中，供后续处理使用
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// WebSocket 授权验证 (同步版本，仅验证JWT)
pub fn verify_ws_token_jwt(token: &str) -> Result<Claims, AppError> {
    // 验证 JWT
    let claims = verify_token(token)?;
    Ok(claims)
}

/// WebSocket 授权验证 (完整版本，包含数据库验证)
pub async fn verify_ws_token(token: &str, state: &AppState) -> Result<Claims, AppError> {
    // 验证 JWT
    let claims = verify_token(token)?;
    
    // 验证 token 是否在数据库中存在且未过期
    let token_hash = generate_token_hash(token);
    if !token_ops::verify_token_exists(&state.db, &token_hash).await? {
        return Err(AppError::AuthError("Token not found or expired".to_string()));
    }
    
    Ok(claims)
}

/// 检查 Token 是否存在 (异步版本)
pub async fn check_token_exists(token: &str, state: &AppState) -> Result<bool, AppError> {
    let token_hash = generate_token_hash(token);
    token_ops::verify_token_exists(&state.db, &token_hash).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_hash() {
        let token = "test_token";
        let hash1 = generate_token_hash(token);
        let hash2 = generate_token_hash(token);
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        let request = axum::http::Request::builder()
            .header(AUTHORIZATION, "Bearer test_token")
            .body(axum::body::Body::empty())
            .unwrap();
            
        let result = extract_bearer_token(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, "test_token");
    }

    #[test]
    fn test_extract_bearer_token_missing_header() {
        let request = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let request = axum::http::Request::builder()
            .header(AUTHORIZATION, "Basic test_token")
            .body(axum::body::Body::empty())
            .unwrap();
            
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_and_verify_token() {
        // 这个测试需要数据库连接，暂时跳过
        // 可以集成到更大的测试套件中
    }
}
