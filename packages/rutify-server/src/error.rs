use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use sea_orm::DbErr;
use tracing::error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum AppError {
    Db(DbErr),
    Json(serde_json::Error),
    AuthError(String),
    DatabaseError(String),
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

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(err) => write!(f, "Database error: {}", err),
            AppError::Json(err) => write!(f, "JSON error: {}", err),
            AppError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database operation error: {}", msg),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::Db(err) => {
                error!(error = %err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".to_string())
            }
            AppError::Json(err) => {
                error!(error = %err, "json error");
                (StatusCode::BAD_REQUEST, "json error".to_string())
            }
            AppError::AuthError(msg) => {
                error!(error = %msg, "authentication error");
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            AppError::DatabaseError(msg) => {
                error!(error = %msg, "database operation error");
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}