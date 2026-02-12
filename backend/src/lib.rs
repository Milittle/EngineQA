pub mod api;
pub mod config;
pub mod indexer;
pub mod observability;
pub mod provider;
pub mod rag;

use crate::provider::InternalApiProvider;

pub fn create_app(
    config: &config::AppConfig,
    provider: InternalApiProvider,
    retriever: rag::VectorRetriever,
) -> axum::Router {
    let router = api::router(config);

    router
        .route(
            "/api/query",
            axum::routing::post(api::query::handle_query::<InternalApiProvider>),
        )
        .axum::extension(config.clone())
        .with_state(provider)
        .with_state(retriever)
}
