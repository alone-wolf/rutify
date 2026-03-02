use crate::db::tokens::{self, Entity as Tokens, Model as TokenModel, TokenType};
use crate::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

pub async fn create_notify_token(
    db: &DatabaseConnection,
    token_hash: &str,
    usage: &str,
    expires_at: chrono::DateTime<Utc>,
    device_info: Option<String>,
) -> Result<TokenModel, AppError> {
    let new_token = tokens::ActiveModel {
        token_hash: Set(token_hash.to_string()),
        usage: Set(usage.to_string()),
        token_type: Set(TokenType::NotifyBearer),
        user_id: Set(None),
        device_info: Set(device_info),
        created_at: Set(Utc::now()),
        expires_at: Set(expires_at),
        last_used_at: Set(None),
        ..Default::default()
    };

    new_token
        .insert(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create notify token: {e}")))
}

pub async fn create_user_token(
    db: &DatabaseConnection,
    token_hash: &str,
    user_id: Uuid,
    expires_at: chrono::DateTime<Utc>,
) -> Result<TokenModel, AppError> {
    let new_token = tokens::ActiveModel {
        token_hash: Set(token_hash.to_string()),
        usage: Set("user_auth".to_string()),
        token_type: Set(TokenType::UserJwt),
        user_id: Set(Some(user_id)),
        device_info: Set(None),
        created_at: Set(Utc::now()),
        expires_at: Set(expires_at),
        last_used_at: Set(None),
        ..Default::default()
    };

    new_token
        .insert(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create user token: {e}")))
}

pub async fn verify_token_exists(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<bool, AppError> {
    let token = Tokens::find()
        .filter(tokens::Column::TokenHash.eq(token_hash))
        .filter(tokens::Column::ExpiresAt.gt(Utc::now()))
        .one(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to verify token: {e}")))?;

    Ok(token.is_some())
}

pub async fn update_token_last_used(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<(), AppError> {
    let token = Tokens::find()
        .filter(tokens::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to find token: {e}")))?;

    if let Some(token) = token {
        let mut active_model: tokens::ActiveModel = token.into();
        active_model.last_used_at = Set(Some(Utc::now()));
        active_model.update(db).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to update token last used: {e}"))
        })?;
    }

    Ok(())
}

pub async fn cleanup_expired_tokens(db: &DatabaseConnection) -> Result<u64, AppError> {
    let result = Tokens::delete_many()
        .filter(tokens::Column::ExpiresAt.lt(Utc::now()))
        .exec(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to cleanup expired tokens: {e}")))?;

    Ok(result.rows_affected)
}

pub async fn get_user_tokens(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<TokenModel>, AppError> {
    Tokens::find()
        .filter(tokens::Column::UserId.eq(Some(user_id)))
        .order_by_desc(tokens::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get user tokens: {e}")))
}

pub async fn delete_token_by_id(db: &DatabaseConnection, token_id: i32) -> Result<bool, AppError> {
    let result = Tokens::delete_by_id(token_id)
        .exec(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete token: {e}")))?;

    Ok(result.rows_affected > 0)
}

pub async fn delete_token_by_hash(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<u64, AppError> {
    let result = Tokens::delete_many()
        .filter(tokens::Column::TokenHash.eq(token_hash))
        .exec(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete token: {e}")))?;

    Ok(result.rows_affected)
}

pub async fn delete_user_tokens(db: &DatabaseConnection, user_id: Uuid) -> Result<u64, AppError> {
    let result = Tokens::delete_many()
        .filter(tokens::Column::UserId.eq(Some(user_id)))
        .exec(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete user tokens: {e}")))?;

    Ok(result.rows_affected)
}

pub async fn list_all_tokens(db: &DatabaseConnection) -> Result<Vec<TokenModel>, AppError> {
    Tokens::find()
        .order_by_desc(tokens::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list all tokens: {e}")))
}

pub async fn list_tokens_by_usage(
    db: &DatabaseConnection,
    usage: &str,
) -> Result<Vec<TokenModel>, AppError> {
    Tokens::find()
        .filter(tokens::Column::Usage.eq(usage))
        .order_by_desc(tokens::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list tokens by usage: {e}")))
}

pub async fn list_tokens_by_type(
    db: &DatabaseConnection,
    token_type: TokenType,
) -> Result<Vec<TokenModel>, AppError> {
    Tokens::find()
        .filter(tokens::Column::TokenType.eq(token_type))
        .order_by_desc(tokens::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to list tokens by type: {e}")))
}
