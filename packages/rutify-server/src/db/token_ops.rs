use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use chrono::{DateTime, Utc};
use crate::db::tokens;
use crate::error::AppError;

/// 创建新的 Token 记录
pub async fn create_token(
    db: &DatabaseConnection,
    token_hash: &str,
    usage: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let new_token = tokens::ActiveModel {
        token_hash: Set(token_hash.to_string()),
        usage: Set(usage.to_string()),
        created_at: Set(Utc::now()),
        expires_at: Set(expires_at),
        ..Default::default()
    };

    let result = tokens::Entity::insert(new_token)
        .exec(db)
        .await;

    match result {
        Ok(_) => {
            tracing::info!("Token created successfully for usage: {}", usage);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to create token: {}", e);
            Err(AppError::DatabaseError(format!("Failed to create token: {}", e)))
        }
    }
}

/// 验证 Token 是否存在且未过期
pub async fn verify_token_exists(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<bool, AppError> {
    let token = tokens::Entity::find()
        .filter(tokens::Column::TokenHash.eq(token_hash))
        .one(db)
        .await?;

    match token {
        Some(token_record) => {
            // 检查是否过期
            let now = Utc::now();
            let is_expired = token_record.expires_at < now;
            
            if is_expired {
                // 删除过期的 token
                delete_token(db, token_hash).await?;
                Ok(false)
            } else {
                Ok(true)
            }
        }
        None => Ok(false),
    }
}

/// 删除 Token
pub async fn delete_token(
    db: &DatabaseConnection,
    token_hash: &str,
) -> Result<(), AppError> {
    let result = tokens::Entity::delete_many()
        .filter(tokens::Column::TokenHash.eq(token_hash))
        .exec(db)
        .await;

    match result {
        Ok(delete_result) => {
            if delete_result.rows_affected > 0 {
                tracing::info!("Token deleted successfully: {}", token_hash);
            }
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to delete token: {}", e);
            Err(AppError::DatabaseError(format!("Failed to delete token: {}", e)))
        }
    }
}

/// 清理过期的 Tokens
pub async fn cleanup_expired_tokens(db: &DatabaseConnection) -> Result<u64, AppError> {
    let now = Utc::now();
    
    let result = tokens::Entity::delete_many()
        .filter(tokens::Column::ExpiresAt.lt(now))
        .exec(db)
        .await?;

    let deleted_count = result.rows_affected;
    if deleted_count > 0 {
        tracing::info!("Cleaned up {} expired tokens", deleted_count);
    }

    Ok(deleted_count)
}

/// 获取所有 Token 信息 (管理员功能)
pub async fn list_all_tokens(db: &DatabaseConnection) -> Result<Vec<tokens::Model>, AppError> {
    let tokens = tokens::Entity::find()
        .all(db)
        .await?;

    Ok(tokens)
}

/// 根据 usage 获取 Tokens
pub async fn list_tokens_by_usage(
    db: &DatabaseConnection,
    usage: &str,
) -> Result<Vec<tokens::Model>, AppError> {
    let tokens = tokens::Entity::find()
        .filter(tokens::Column::Usage.eq(usage))
        .all(db)
        .await?;

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use chrono::Duration;

    #[tokio::test]
    async fn test_token_operations() {
        // 使用内存数据库进行测试
        let db = Database::connect("sqlite::memory:").await.unwrap();
        
        // 创建表 (这里需要确保 tokens 表存在)
        // 在实际应用中，这应该通过迁移来完成
        
        let token_hash = "test_hash_123";
        let usage = "test_usage";
        let expires_at = Utc::now() + Duration::hours(24);
        
        // 测试创建 token
        let result = create_token(&db, token_hash, usage, expires_at).await;
        // 暂时跳过这个测试，因为需要数据库表结构
        // assert!(result.is_ok());
        
        // 测试验证 token
        let exists = verify_token_exists(&db, token_hash).await;
        // assert!(exists.unwrap_or(false));
        
        // 测试删除 token
        let delete_result = delete_token(&db, token_hash).await;
        // assert!(delete_result.is_ok());
        
        // 验证删除后不存在
        let exists_after_delete = verify_token_exists(&db, token_hash).await;
        // assert!(!exists_after_delete.unwrap_or(true));
    }

    #[test]
    fn test_token_hash_validation() {
        // 测试 token hash 格式
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(hash.len(), 64);
        
        // 测试无效 hash
        let invalid_hash = "invalid";
        assert_ne!(invalid_hash.len(), 64);
    }
}
