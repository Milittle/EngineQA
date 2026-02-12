pub mod api;
pub mod config;
pub mod indexer;
pub mod observability;
pub mod provider;
pub mod rag;

use crate::{
    api::reindex::JobManager,
    indexer::MarkdownIndexer,
    provider::InternalApiProvider,
};

pub fn create_app(
    config: &config::AppConfig,
    provider: InternalApiProvider,
    retriever: rag::VectorRetriever,
) -> axum::Router {
    let router = api::router(config);

    // Initialize indexer
    let indexer = match MarkdownIndexer::new(
        config.internal_api.clone(),
        &config.qdrant_url,
        &config.knowledge_dir,
    ) {
        Ok(indexer) => indexer,
        Err(e) => {
            tracing::warn!(error = %e, "failed to initialize indexer, reindex will be unavailable");
            // We still return the router, but reindex will fail when called
            panic!("Indexer initialization failed: {}", e);
        }
    };

    // Initialize job manager
    let job_manager = JobManager::new();

    // Initialize Qdrant client for status endpoint
    let qdrant_client = qdrant_client::Qdrant::from_url(&config.qdrant_url)
        .build()
        .expect("Failed to create Qdrant client");

    router
        .route(
            "/api/query",
            axum::routing::post(api::query::handle_query::<InternalApiProvider>),
        )
        .route(
            "/api/reindex",
            axum::routing::post(api::reindex::handle_reindex)
                .get(api::reindex::handle_reindex_status),
        )
        .route(
            "/api/status",
            axum::routing::get(api::status::handle_status),
        )
        .axum::extension(config.clone())
        .with_state(provider)
        .with_state(retriever)
        .with_state(indexer)
        .with_state(job_manager)
        .with_state(qdrant_client)
}
