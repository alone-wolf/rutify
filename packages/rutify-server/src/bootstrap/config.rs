use anyhow::{Context, Result};
use common_http_server_rs::{AppConfig, CorsConfig, LogFormat, LoggingConfig, ServerConfig};
use std::net::SocketAddr;

const DEFAULT_ADDR: &str = "0.0.0.0:3000";

pub(crate) fn server_config_from_env() -> Result<ServerConfig> {
    let addr_text = std::env::var("RUTIFY_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let addr: SocketAddr = addr_text
        .parse()
        .with_context(|| format!("invalid RUTIFY_ADDR: {addr_text}"))?;

    Ok(ServerConfig::new(addr.port()).with_host(addr.ip().to_string()))
}

pub(crate) fn app_config_from_env() -> AppConfig {
    let cors_config = CorsConfig::from_env();
    let logging_config = LoggingConfig::default()
        .with_format(LogFormat::Pretty)
        .with_json_backend(false);

    AppConfig::new()
        .with_cors_config(cors_config)
        .with_logging(true)
        .with_logging_config(logging_config)
        .with_tracing(true)
}
