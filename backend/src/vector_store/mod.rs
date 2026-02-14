use async_trait::async_trait;
use std::collections::HashMap;

pub mod lancedb_store;

#[derive(Debug, Clone)]
pub struct StoredChunk {
    pub point_id: String,
    pub doc_id: String,
    pub chunk_id: String,
    pub path: String,
    pub title_path: String,
    pub section: String,
    pub text: String,
    pub hash: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub doc_id: String,
    pub path: String,
    pub title_path: String,
    pub section: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("LanceDB error: {0}")]
    LanceDb(#[from] lancedb::Error),

    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),

    #[error("Invalid vector store payload: {0}")]
    InvalidPayload(String),
}

pub type VectorStoreResult<T> = Result<T, VectorStoreError>;

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn ensure_ready(&self) -> VectorStoreResult<()>;

    async fn search(&self, query_vector: Vec<f32>, top_k: u64)
    -> VectorStoreResult<Vec<SearchHit>>;

    async fn upsert_chunks(&self, chunks: Vec<StoredChunk>) -> VectorStoreResult<()>;

    async fn delete_by_doc_id(&self, doc_id: &str) -> VectorStoreResult<()>;

    async fn list_doc_hashes(&self) -> VectorStoreResult<HashMap<String, String>>;

    async fn count(&self) -> VectorStoreResult<usize>;
}
