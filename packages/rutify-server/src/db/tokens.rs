use chrono::Utc;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum")]
pub enum TokenType {
    #[sea_orm(string_value = "user_jwt")]
    UserJwt,
    #[sea_orm(string_value = "notify_bearer")]
    NotifyBearer,
}

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment_flag)]
    pub id: i32,
    pub token_hash: String,
    pub usage: String,
    pub token_type: TokenType,
    pub user_id: Option<Uuid>,
    pub device_info: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub last_used_at: Option<chrono::DateTime<Utc>>,
}

impl ActiveModelBehavior for ActiveModel {}
