use crate::db;
use sea_orm::sea_query::Table;
use sea_orm::{DbErr, DeriveMigrationName};
use sea_orm_migration::{MigrationTrait, SchemaManager, schema};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 notifies 表
        let notifies_table = Table::create()
            .table(db::Notifies)
            .if_not_exists()
            .col(schema::pk_auto(db::Notifies::COLUMN.id))
            .col(schema::string(db::Notifies::COLUMN.notify))
            .col(schema::string(db::Notifies::COLUMN.device))
            .col(schema::string(db::Notifies::COLUMN.title))
            .col(schema::date(db::Notifies::COLUMN.received_at))
            .to_owned();

        // 创建 tokens 表（包含所有必要的列）
        let tokens_table = Table::create()
            .table(db::Tokens)
            .if_not_exists()
            .col(schema::pk_auto(db::Tokens::COLUMN.id))
            .col(schema::string(db::Tokens::COLUMN.token_hash))
            .col(schema::string(db::Tokens::COLUMN.usage))
            .col(schema::string("token_type").default("notify_bearer"))
            .col(schema::uuid("user_id").null())
            .col(schema::string("device_info").null())
            .col(schema::date(db::Tokens::COLUMN.created_at))
            .col(schema::date(db::Tokens::COLUMN.expires_at))
            .col(schema::date("last_used_at").null())
            .to_owned();

        // 创建 users 表
        let users_table = Table::create()
            .table(db::Users)
            .if_not_exists()
            .col(schema::uuid(db::Users::COLUMN.id).primary_key())
            .col(schema::string(db::Users::COLUMN.username))
            .col(schema::string(db::Users::COLUMN.password_hash))
            .col(schema::string(db::Users::COLUMN.email))
            .col(schema::string(db::Users::COLUMN.role))
            .col(schema::date(db::Users::COLUMN.created_at))
            .col(schema::date(db::Users::COLUMN.updated_at))
            .to_owned();

        // 依次创建所有表
        manager.create_table(notifies_table).await?;
        manager.create_table(tokens_table).await?;
        manager.create_table(users_table).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 简化开发阶段，不需要回滚逻辑
        Ok(())
    }
}
