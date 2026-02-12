use tokio::net::TcpListener;

use engineqa_backend::{
    config::AppConfig,
    create_app,
    observability,
    provider::InternalApiProvider,
    rag::VectorRetriever,
};

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
        qdrant_url = %config.qdrant_url,
        "configuration loaded"
    );

    // Initialize provider
    let provider = InternalApiProvider::new(config.internal_api.clone());

    // Initialize retriever
    let retriever = match VectorRetriever::new(&config.qdrant_url) {
        Ok(retriever) => retriever,
        Err(err) => {
            tracing::error!(error = %err, "failed to initialize Qdrant retriever");
            std::process::exit(1);
        }
    };

    // Ensure collection exists
    if let Err(err) = retriever.ensure_collection_exists().await {
        tracing::warn!(error = %err, "failed to ensure Qdrant collection exists");
    }

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

    let app = create_app(&config, provider, retriever);

    tracing::info!(address = %addr, "backend started");

    if let Err(err) = axum::serve(listener, app).await {
        tracing::error!(error = %err, "backend server exited with error");
        std::process::exit(1);
    }
}
