use rutify_core::NotificationData;
use chrono::Utc;
use sea_orm::ActiveValue;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notifies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment_flag)]
    pub id: i32,
    pub notify: String,
    pub title: Option<String>,
    pub device: Option<String>,
    pub received_at: chrono::DateTime<Utc>,
}

impl ActiveModelBehavior for ActiveModel {}

pub(crate) async fn insert_new_notify(db: &DatabaseConnection, data: NotificationData) {
    let received_at = Utc::now();

    ActiveModel {
        id: ActiveValue::NotSet,
        notify: ActiveValue::Set(data.notify),
        title: ActiveValue::Set(Some(data.title)),
        device: ActiveValue::Set(Some(data.device)),
        received_at: ActiveValue::Set(received_at),
    }
    .insert(db)
    .await
    .unwrap();
}
