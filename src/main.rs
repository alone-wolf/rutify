mod app;
mod db;
mod error;
mod payload;
mod routes;
mod state;

use crate::payload::NotifyEvent;
use crate::state::AppState;
use dotenvy::dotenv;
use sea_orm::{Database, DbErr};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    dotenv().ok();
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let db_url = std::env::var("RUTIFY_DB_URL")
        .unwrap_or_else(|_| "sqlite://rutify.db?mode=rwc".to_string());
    let db_cnn = Database::connect(&db_url).await?;
    db::initialize::initial(&db_cnn).await;

    let (tx, _) = broadcast::channel(200);
    let state = Arc::new(AppState { db: db_cnn, tx });

    let app = app::axum_app(state);

    let addr: SocketAddr = std::env::var("RUTIFY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("invalid RUTIFY_ADDR");

    info!(%addr, "rutify started");
    println!(
        "rutify started at http://{}",
        addr.clone().to_string().replace("0.0.0.0", "127.0.0.1")
    );
    let tcp_listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(tcp_listener, app).await.unwrap();
    Ok(())
}
