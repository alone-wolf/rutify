use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use std::sync::Arc;

use crate::services::auth::auth::{create_token, delete_token, get_tokens};
use crate::services::auth::user::{
    get_user_profile, login_user, register_user, user_auth_middleware,
};
use crate::state::AppState;

pub(crate) fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected_router = Router::new()
        .route("/profile", get(get_user_profile))
        .route("/tokens", post(create_token))
        .route("/tokens", get(get_tokens))
        .route("/tokens/{id}", delete(delete_token))
        .layer(middleware::from_fn_with_state(state, user_auth_middleware));

    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login_user))
        .merge(protected_router)
}

// /// Token信息响应
// #[derive(serde::Serialize)]
// pub struct TokenInfoResponse {
//     pub id: i32,
//     pub usage: String,
//     pub token_type: String,
//     pub device_info: Option<String>,
//     pub created_at: String,
//     pub expires_at: String,
//     pub last_used_at: Option<String>,
// }
