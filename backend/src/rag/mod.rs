use std::sync::Arc;

use crate::vector_store::{VectorStore, VectorStoreError};

const DEFAULT_TOP_K: u64 = 6;

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub doc_id: String,
    pub path: String,
    pub title_path: String,
    pub section: String,
}

#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    pub metadata: ChunkMetadata,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum RetrieverError {
    #[error("Vector store error: {0}")]
    VectorStoreError(#[from] VectorStoreError),

    #[error("Score too low for all results")]
    NoResultsAboveThreshold,
}

pub type RetrieverResult<T> = Result<T, RetrieverError>;

#[derive(Clone)]
pub struct VectorRetriever {
    store: Arc<dyn VectorStore>,
    score_threshold: f32,
}

impl VectorRetriever {
    pub fn new(store: Arc<dyn VectorStore>, score_threshold: f32) -> Self {
        Self {
            store,
            score_threshold,
        }
    }

    pub async fn retrieve(
        &self,
        query_vector: Vec<f32>,
        top_k: Option<u64>,
    ) -> RetrieverResult<Vec<RetrievedChunk>> {
        let top_k = top_k.unwrap_or(DEFAULT_TOP_K);
        let hits = self.store.search(query_vector, top_k).await?;

        if hits.is_empty() {
            return Ok(vec![]);
        }

        let chunks: Vec<RetrievedChunk> = hits
            .into_iter()
            .filter(|hit| hit.score >= self.score_threshold)
            .map(|hit| RetrievedChunk {
                metadata: ChunkMetadata {
                    doc_id: hit.doc_id,
                    path: hit.path,
                    title_path: hit.title_path,
                    section: hit.section,
                },
                snippet: hit.snippet,
                score: hit.score,
            })
            .collect();

        if chunks.is_empty() {
            return Err(RetrieverError::NoResultsAboveThreshold);
        }

        Ok(chunks)
    }

    pub async fn ensure_collection_exists(&self) -> RetrieverResult<()> {
        self.store.ensure_ready().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_top_k_constant() {
        assert_eq!(DEFAULT_TOP_K, 6);
    }
}
