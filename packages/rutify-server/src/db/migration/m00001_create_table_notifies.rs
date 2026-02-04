use sea_orm::{DbErr, DeriveMigrationName};
use sea_orm::sea_query::Table;
use sea_orm_migration::{schema, MigrationTrait, SchemaManager};
use crate::db;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Table::create()
            .table(db::Notifies)
            .if_not_exists()
            .col(schema::pk_auto(db::Notifies::COLUMN.id))
            .col(schema::string(db::Notifies::COLUMN.notify))
            .col(schema::string(db::Notifies::COLUMN.device))
            .col(schema::string(db::Notifies::COLUMN.title))
            .col(schema::date(db::Notifies::COLUMN.received_at))
            .to_owned();
        manager.create_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(db::Notifies).to_owned())
            .await
    }
}