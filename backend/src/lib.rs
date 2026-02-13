pub mod api;
pub mod config;
pub mod indexer;
pub mod observability;
pub mod provider;
pub mod rag;

use std::sync::Arc;

use crate::{
    api::feedback::FeedbackStore,
    api::reindex::JobManager,
    indexer::MarkdownIndexer,
    provider::InternalApiProvider,
};

pub struct AppState {
    pub config: config::AppConfig,
    pub provider: InternalApiProvider,
    pub retriever: rag::VectorRetriever,
    pub indexer: MarkdownIndexer,
    pub job_manager: JobManager,
    pub qdrant_client: qdrant_client::Qdrant,
    pub feedback_store: FeedbackStore,
}

pub fn create_app(
    config: &config::AppConfig,
    provider: InternalApiProvider,
    retriever: rag::VectorRetriever,
) -> axum::Router {
    let router = api::router::<Arc<AppState>>(config);

    // Initialize indexer
    let indexer = match MarkdownIndexer::new(
        config.internal_api.clone(),
        &config.qdrant_url,
        &config.knowledge_dir,
    ) {
        Ok(indexer) => indexer,
        Err(e) => {
            tracing::warn!(error = %e, "failed to initialize indexer, reindex will be unavailable");
            panic!("Indexer initialization failed: {}", e);
        }
    };

    // Initialize job manager
    let job_manager = JobManager::new();

    // Initialize Qdrant client for status endpoint
    let qdrant_client = qdrant_client::Qdrant::from_url(&config.qdrant_url)
        .build()
        .expect("Failed to create Qdrant client");

    // Initialize feedback store
    let feedback_store = FeedbackStore::new();

    let state = Arc::new(AppState {
        config: config.clone(),
        provider,
        retriever,
        indexer,
        job_manager,
        qdrant_client,
        feedback_store,
    });

    router
        .route(
            "/api/query",
            axum::routing::post(api::query::handle_query),
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
        .route(
            "/api/feedback",
            axum::routing::post(api::feedback::handle_feedback),
        )
        .with_state(state)
}
