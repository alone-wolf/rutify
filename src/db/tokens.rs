use chrono::Utc;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment_flag)]
    pub id: i32,
    pub token_hash: String,
    pub usage:String,
    pub created_at: chrono::DateTime<Utc>,
}

impl ActiveModelBehavior for ActiveModel {}
