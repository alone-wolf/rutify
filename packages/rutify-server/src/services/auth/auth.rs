use axum::{
    Json,
    extract::{Path, Request, State},
    http::StatusCode,
    http::header::AUTHORIZATION,
    middleware::Next,
    response::IntoResponse,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::token_ops;
use crate::error::AppError;
use crate::state::AppState;

// /notify 使用key走bearer token
// /api 使用 jwt

/// JWT Claims 结构 (用于通知Token)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Token ID
    pub usage: String,      // Token用途
    pub token_type: String, // Token type (notify_bearer)
    pub iat: i64,           // 签发时间
    pub exp: i64,           // 过期时间
    pub jti: String,        // JWT ID
}

/// JWT Claims 结构 (用于通知Token)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub sub: String,        // Token ID
    pub usage: String,      // Token用途
    pub token_type: String, // Token type (notify_bearer)
    pub iat: i64,           // 签发时间
    pub exp: i64,           // 过期时间
    pub jti: String,        // JWT ID
}

/// Token 创建请求
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub usage: String,
    pub expires_in_hours: Option<u64>,
    pub device_info: Option<String>,
}

/// Token 创建响应
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub token_id: String,
    pub usage: String,
    pub token_type: String,
    pub expires_at: String,
}

/// Token信息响应
#[derive(Debug, Serialize)]
pub struct TokenInfoResponse {
    pub id: i32,
    pub usage: String,
    pub token_type: String,
    pub device_info: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
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

/// 创建新的通知 JWT Token
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, AppError> {
    let token_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let expires_in = request.expires_in_hours.unwrap_or(24); // 默认24小时
    let expires_at = now + chrono::Duration::hours(expires_in as i64);

    let claims = TokenClaims {
        sub: token_id.clone(),
        usage: request.usage.clone(),
        token_type: "notify_bearer".to_string(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };

    let secret = get_jwt_secret();

    // 明确指定HS256算法
    let header = Header::new(jsonwebtoken::Algorithm::HS256);

    let token =
        encode(&header, &claims, &EncodingKey::from_secret(secret.as_ref())).map_err(|e| {
            error!("Failed to encode JWT: {}", e);
            AppError::AuthError("Failed to create token".to_string())
        })?;

    // 保存 token hash 到数据库
    let token_hash = generate_token_hash(&token);
    token_ops::create_notify_token(
        &state.db,
        &token_hash,
        &request.usage,
        expires_at,
        request.device_info,
    )
    .await?;

    info!("Created new notify token for usage: {}", request.usage);

    Ok(Json(CreateTokenResponse {
        token,
        token_id,
        usage: request.usage,
        token_type: "notify_bearer".to_string(),
        expires_at: expires_at.to_string(),
    }))
}

pub async fn get_tokens(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let data = token_ops::list_all_tokens(&state.db).await?;
    let tokens: Vec<TokenInfoResponse> = data
        .into_iter()
        .map(|item| TokenInfoResponse {
            id: item.id,
            usage: item.usage,
            token_type: match item.token_type {
                crate::db::tokens::TokenType::UserJwt => "user_jwt".to_string(),
                crate::db::tokens::TokenType::NotifyBearer => "notify_bearer".to_string(),
            },
            device_info: item.device_info,
            created_at: item.created_at.to_string(),
            expires_at: item.expires_at.to_string(),
            last_used_at: item.last_used_at.map(|dt| dt.to_string()),
        })
        .collect();
    Ok((StatusCode::OK, Json(tokens)))
}

pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = token_ops::delete_token_by_id(&state.db, token_id).await?;
    if deleted {
        Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
    } else {
        Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "errors": "Token not found" })),
        ))
    }
}

/// 验证通知 JWT Token
pub fn verify_notify_token(token: &str) -> Result<TokenClaims, AppError> {
    let secret = get_jwt_secret();

    // 创建严格的验证配置
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true; // 验证过期时间
    validation.leeway = 60; // 允许60秒的时钟偏差

    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )
    .map_err(|e| {
        error!("Notify JWT verification failed: {}", e);
        AppError::AuthError("Invalid notify token".to_string())
    })?;

    // 验证token类型
    if token_data.claims.token_type != "notify_bearer" {
        return Err(AppError::AuthError("Invalid token type".to_string()));
    }

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
        return Err(AppError::AuthError(
            "Invalid authorization header format".to_string(),
        ));
    }

    let token = auth_header.trim_start_matches("Bearer ").to_string();
    Ok(BearerToken(token))
}

/// 通知Token授权中间件
pub async fn notify_token_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let BearerToken(token) = extract_bearer_token(&request)?;

    // 验证 JWT
    let claims = verify_notify_token(&token)?;

    // 验证 token 是否在数据库中存在且未过期
    let token_hash = generate_token_hash(&token);
    if !token_ops::verify_token_exists(&state.db, &token_hash).await? {
        return Err(AppError::AuthError(
            "Token not found or expired".to_string(),
        ));
    }

    // 更新最后使用时间
    token_ops::update_token_last_used(&state.db, &token_hash).await?;

    // 将 claims 添加到请求扩展中，供后续处理使用
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// WebSocket 授权验证 (同步版本，仅验证JWT)
pub fn verify_ws_token_jwt(token: &str) -> Result<TokenClaims, AppError> {
    // 验证 JWT
    let claims = verify_notify_token(token)?;
    Ok(claims)
}

/// WebSocket 授权验证 (完整版本，包含数据库验证)
pub async fn verify_ws_token(token: &str, state: &AppState) -> Result<TokenClaims, AppError> {
    // 验证 JWT
    let claims = verify_notify_token(token)?;

    // 验证 token 是否在数据库中存在且未过期
    let token_hash = generate_token_hash(token);
    if !token_ops::verify_token_exists(&state.db, &token_hash).await? {
        return Err(AppError::AuthError(
            "Token not found or expired".to_string(),
        ));
    }

    Ok(claims)
}

/// 检查 Token 是否存在 (异步版本)
pub async fn check_token_exists(token: &str, state: &AppState) -> Result<bool, AppError> {
    let token_hash = generate_token_hash(token);
    token_ops::verify_token_exists(&state.db, &token_hash).await
}
