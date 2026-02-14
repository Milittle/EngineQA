use tokio::net::TcpListener;

use engineqa_backend::{
    config::AppConfig, create_app, observability, provider::InternalApiProvider,
    rag::VectorRetriever, vector_store::lancedb_store::LanceDbStore,
};
use std::sync::Arc;

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
        vector_store = %config.vector_store,
        lancedb_uri = %config.lancedb_uri,
        lancedb_table = %config.lancedb_table,
        "configuration loaded"
    );

    // Initialize provider
    let provider = InternalApiProvider::new(config.internal_api.clone());

    // Initialize vector store
    let vector_store = match LanceDbStore::new(
        &config.lancedb_uri,
        &config.lancedb_table,
        config.embedding_vector_size,
    )
    .await
    {
        Ok(store) => Arc::new(store),
        Err(err) => {
            tracing::error!(error = %err, "failed to initialize LanceDB vector store");
            std::process::exit(1);
        }
    };

    // Initialize retriever
    let retriever = VectorRetriever::new(vector_store.clone(), config.vector_score_threshold);

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

    let app = create_app(&config, provider, retriever, vector_store);

    tracing::info!(address = %addr, "backend started");

    if let Err(err) = axum::serve(listener, app).await {
        tracing::error!(error = %err, "backend server exited with error");
        std::process::exit(1);
    }
}
