use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get};
use axum::{Json, Router};
use rutify_core::NotifyItem;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use std::sync::Arc;

pub(crate) fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_notifies_handler))
        .route("/", delete(delete_all_notifies_handler))
        .route("/{id}", delete(delete_notify_by_id_handler))
}

async fn delete_all_notifies_handler(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = crate::db::notifies::Entity::delete_many()
        .exec(&state.db)
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "data": {
                "deleted_count": deleted.rows_affected
            }
        })),
    ))
}

async fn delete_notify_by_id_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = crate::db::notifies::Entity::delete_by_id(id)
        .exec(&state.db)
        .await?;

    if deleted.rows_affected == 0 {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "errors": "Notify not found"
            })),
        ));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

async fn list_notifies_handler(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let total = crate::db::notifies::Entity::find().count(&state.db).await?;
    let notifies = crate::db::notifies::Entity::find()
        .order_by_desc(crate::db::notifies::Column::ReceivedAt)
        .all(&state.db)
        .await?;

    let data: Vec<NotifyItem> = notifies
        .into_iter()
        .map(|item| NotifyItem {
            id: item.id,
            title: item.title.unwrap_or_else(|| "default title".to_string()),
            notify: item.notify,
            device: item.device.unwrap_or_else(|| "default device".to_string()),
            received_at: item.received_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "data": data,
            "meta": {
                "total": total
            }
        })),
    ))
}
