pub mod api;
pub mod config;
pub mod indexer;
pub mod observability;
pub mod provider;
pub mod rag;

pub fn app() -> axum::Router {
    api::router()
}
