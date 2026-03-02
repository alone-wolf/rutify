use axum::{
    Extension, Json,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use sea_orm::{ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::tokens::{self, Entity as Tokens, TokenType};
use crate::db::users::{
    self, ActiveModel as UserActiveModel, Entity as Users, Model as UserModel, UserRole,
};
use crate::error::AppError;
use crate::state::AppState;

/// 用户登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 用户注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

/// 用户登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub jwt_token: String,
    pub expires_at: String,
}

/// 用户信息响应
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: String,
}

/// 用户JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserClaims {
    pub sub: String,        // User ID
    pub username: String,   // Username
    pub role: UserRole,     // User role
    pub iat: i64,           // 签发时间
    pub exp: i64,           // 过期时间
    pub jti: String,        // JWT ID
    pub token_type: String, // Token type (user_jwt)
}

/// 基础认证提取器
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

/// JWT Token提取器
pub struct UserJwt(pub UserClaims);

/// 从请求头中提取Basic Auth
pub fn extract_basic_auth(request: &Request) -> Result<BasicAuth, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| AppError::AuthError("Missing authorization header".to_string()))?;

    if !auth_header.starts_with("Basic ") {
        return Err(AppError::AuthError(
            "Invalid authorization header format".to_string(),
        ));
    }

    let encoded = auth_header.trim_start_matches("Basic ");
    let decoded = base64::decode(encoded)
        .map_err(|_| AppError::AuthError("Invalid base64 encoding".to_string()))?;

    let credentials = String::from_utf8(decoded)
        .map_err(|_| AppError::AuthError("Invalid UTF-8 encoding".to_string()))?;

    let mut parts = credentials.splitn(2, ':');
    let username = parts
        .next()
        .ok_or_else(|| AppError::AuthError("Missing username".to_string()))?
        .to_string();
    let password = parts
        .next()
        .ok_or_else(|| AppError::AuthError("Missing password".to_string()))?
        .to_string();

    Ok(BasicAuth { username, password })
}

/// 哈希密码
pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash(password, DEFAULT_COST).map_err(|e| {
        error!("Failed to hash password: {}", e);
        AppError::AuthError("Failed to process password".to_string())
    })
}

/// 验证密码
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    verify(password, hash).map_err(|e| {
        error!("Failed to verify password: {}", e);
        AppError::AuthError("Failed to verify password".to_string())
    })
}

/// 创建用户JWT Token
pub fn create_user_jwt_token(user: &UserModel) -> Result<String, AppError> {
    let secret = get_jwt_secret();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(7); // 7天有效期

    let claims = UserClaims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
        jti: Uuid::new_v4().to_string(),
        token_type: "user_jwt".to_string(),
    };

    let header = Header::new(jsonwebtoken::Algorithm::HS256);

    encode(&header, &claims, &EncodingKey::from_secret(secret.as_ref())).map_err(|e| {
        error!("Failed to encode user JWT: {}", e);
        AppError::AuthError("Failed to create user token".to_string())
    })
}

/// 验证用户JWT Token
pub fn verify_user_jwt_token(token: &str) -> Result<UserClaims, AppError> {
    let secret = get_jwt_secret();

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 60;

    let token_data = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )
    .map_err(|e| {
        error!("User JWT verification failed: {}", e);
        AppError::AuthError("Invalid user token".to_string())
    })?;

    // 验证token类型
    if token_data.claims.token_type != "user_jwt" {
        return Err(AppError::AuthError("Invalid token type".to_string()));
    }

    Ok(token_data.claims)
}

/// JWT 密钥
fn get_jwt_secret() -> String {
    let secret = std::env::var("RUTIFY_JWT_SECRET").unwrap_or_else(|_| {
        warn!("Using default JWT secret. Please set RUTIFY_JWT_SECRET environment variable in production!");
        "rutify_default_jwt_secret_change_in_production".to_string()
    });

    if secret.len() < 32 {
        error!("JWT secret is too short (minimum 32 characters required)");
        panic!("JWT secret must be at least 32 characters long");
    }

    secret
}

/// 查找用户的辅助函数
async fn find_user_by_username(
    state: &Arc<AppState>,
    username: &str,
) -> Result<Option<UserModel>, AppError> {
    Users::find()
        .filter(users::Column::Username.eq(username))
        .one(&state.db)
        .await
        .map_err(|e| {
            error!("Database errors finding user: {}", e);
            AppError::DatabaseError("Failed to find user".to_string())
        })
}

/// 根据ID查找用户的辅助函数
async fn find_user_by_id(
    state: &Arc<AppState>,
    user_id: Uuid,
) -> Result<Option<UserModel>, AppError> {
    Users::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            error!("Database errors finding user: {}", e);
            AppError::DatabaseError("Failed to find user".to_string())
        })
}

/// 创建用户响应的辅助函数
fn create_user_response(user: &UserModel) -> UserResponse {
    UserResponse {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        created_at: user.created_at.to_string(),
    }
}

/// 用户注册
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // 检查用户名是否已存在
    let existing_user = find_user_by_username(&state, &request.username).await?;

    if existing_user.is_some() {
        return Err(AppError::AuthError("Username already exists".to_string()));
    }

    // 哈希密码
    let password_hash = hash_password(&request.password)?;

    // 创建用户
    let new_user = UserActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(request.username.clone()),
        password_hash: Set(password_hash),
        email: Set(request.email.clone()),
        role: Set(UserRole::User), // 默认为普通用户
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
    };

    let user = new_user.insert(&state.db).await.map_err(|e| {
        error!("Failed to create user: {}", e);
        AppError::DatabaseError("Failed to create user".to_string())
    })?;

    info!("User registered successfully: {}", user.username);

    Ok(Json(create_user_response(&user)))
}

/// 用户登录
pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 查找用户
    let user = find_user_by_username(&state, &request.username).await?;

    let user =
        user.ok_or_else(|| AppError::AuthError("Invalid username or password".to_string()))?;

    // 验证密码
    let is_valid = verify_password(&request.password, &user.password_hash)?;
    if !is_valid {
        return Err(AppError::AuthError(
            "Invalid username or password".to_string(),
        ));
    }

    // 创建JWT token
    let jwt_token = create_user_jwt_token(&user)?;
    let expires_at = Utc::now() + chrono::Duration::days(7);

    info!("User logged in successfully: {}", user.username);

    Ok(Json(LoginResponse {
        user_id: user.id,
        username: user.username,
        email: user.email,
        role: user.role,
        jwt_token,
        expires_at: expires_at.to_string(),
    }))
}

/// 获取用户信息
pub async fn get_user_profile(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<UserClaims>,
) -> Result<Json<UserResponse>, AppError> {
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::AuthError("Invalid user ID".to_string()))?;

    let user = find_user_by_id(&state, user_id).await?;

    let user = user.ok_or_else(|| AppError::AuthError("User not found".to_string()))?;

    Ok(Json(create_user_response(&user)))
}

/// 用户认证中间件
pub async fn user_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let UserJwt(claims) = extract_user_jwt(&request)?;

    // 验证用户是否仍然存在且活跃
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::AuthError("Invalid user ID".to_string()))?;

    let user = find_user_by_id(&state, user_id).await?;

    let user = user.ok_or_else(|| AppError::AuthError("User not found".to_string()))?;

    // 将用户信息和claims添加到请求扩展中
    request.extensions_mut().insert(user);
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// 从请求中提取用户JWT
pub fn extract_user_jwt(request: &Request) -> Result<UserJwt, AppError> {
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

    let token = auth_header.trim_start_matches("Bearer ");
    let claims = verify_user_jwt_token(token)?;

    Ok(UserJwt(claims))
}
