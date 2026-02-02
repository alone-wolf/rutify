use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use sea_orm::DbErr;
use tracing::error;

#[derive(Debug)]
pub(crate) enum AppError {
    Db(DbErr),
    Json(serde_json::Error),
}

impl From<DbErr> for AppError {
    fn from(err: DbErr) -> Self {
        Self::Db(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let message = match self {
            AppError::Db(err) => {
                error!(error = %err, "database error");
                "database error"
            }
            AppError::Json(err) => {
                error!(error = %err, "json error");
                "json error"
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}