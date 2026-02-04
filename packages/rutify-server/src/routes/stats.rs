use crate::error::AppError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

pub(crate) async fn stats_handler() -> Result<impl IntoResponse, AppError> {
    let data = serde_json::json!({
        "today_count": 5,
        "total_count": 128,
        "device_count": 3,
        "is_running": true
    });

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "data": data
        })),
    ))
}
