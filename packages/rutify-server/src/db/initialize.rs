use crate::db::migration::{m00001_create_table_notifies, m00002_create_table_tokens};
use sea_orm::DbConn;
use sea_orm_migration::{MigrationTrait, MigratorTrait};

pub(crate) async fn initial(db_cnn: &DbConn) {
    Migrator::up(db_cnn, None).await.unwrap();
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_table_notifies::Migration),
            Box::new(m00002_create_table_tokens::Migration),
        ]
    }
}
