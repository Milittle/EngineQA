use tokio::net::TcpListener;

use engineqa_backend::{app, config::AppConfig, observability};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    observability::init();

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(error = %err, "configuration validation failed");
            std::process::exit(1);
        }
    };

    tracing::info!(
        infer_provider = %config.infer_provider,
        upstream_base = %config.internal_api.base_url,
        "configuration loaded"
    );

    let addr = match config.socket_addr() {
        Ok(addr) => addr,
        Err(err) => {
            tracing::error!(error = %err, "invalid backend binding config");
            std::process::exit(1);
        }
    };

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, "failed to bind backend listener");
            std::process::exit(1);
        }
    };

    tracing::info!(address = %addr, "backend started");

    if let Err(err) = axum::serve(listener, app()).await {
        tracing::error!(error = %err, "backend server exited with error");
        std::process::exit(1);
    }
}
