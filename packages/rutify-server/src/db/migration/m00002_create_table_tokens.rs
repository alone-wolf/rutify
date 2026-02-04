use crate::db;
use sea_orm::sea_query::Table;
use sea_orm::{DbErr, DeriveMigrationName};
use sea_orm_migration::{MigrationTrait, SchemaManager, schema};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Table::create()
            .table(db::Tokens)
            .if_not_exists()
            .col(schema::pk_auto(db::Tokens::COLUMN.id))
            .col(schema::string(db::Tokens::COLUMN.token_hash))
            .col(schema::string(db::Tokens::COLUMN.usage))
            .col(schema::date(db::Tokens::COLUMN.created_at))
            .col(schema::date(db::Tokens::COLUMN.expires_at))
            .to_owned();
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(db::Tokens).to_owned())
            .await
    }
}
