use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{value::Kind, Filter, SearchPoints, VectorStruct},
};

const COLLECTION_NAME: &str = "knowledge_chunks";
const DEFAULT_TOP_K: u64 = 6;
const SCORE_THRESHOLD: f32 = 0.3;

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
    #[error("Qdrant client error: {0}")]
    QdrantError(#[from] qdrant_client::QdrantError),

    #[error("Invalid payload format: {0}")]
    InvalidPayload(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Score too low for all results")]
    NoResultsAboveThreshold,
}

pub type RetrieverResult<T> = Result<T, RetrieverError>;

pub struct VectorRetriever {
    client: QdrantClient,
}

impl VectorRetriever {
    pub fn new(qdrant_url: &str) -> RetrieverResult<Self> {
        let config = QdrantClientConfig::from_url(qdrant_url);
        let client = QdrantClient::new(config)?;

        Ok(Self { client })
    }

    pub async fn retrieve(
        &self,
        query_vector: Vec<f32>,
        top_k: Option<u64>,
    ) -> RetrieverResult<Vec<RetrievedChunk>> {
        let top_k = top_k.unwrap_or(DEFAULT_TOP_K);

        let search_result = self
            .client
            .search_points(&SearchPoints {
                collection_name: COLLECTION_NAME.into(),
                vector: Some(VectorStruct::Dense(query_vector.into())),
                limit: top_k,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await?;

        if search_result.result.is_empty() {
            return Ok(vec![]);
        }

        let mut chunks = Vec::new();
        for result in search_result.result {
            if result.score < SCORE_THRESHOLD {
                continue;
            }

            let metadata = self.extract_metadata(&result.payload)?;
            let snippet = self.extract_text(&result.payload)?;

            chunks.push(RetrievedChunk {
                metadata,
                snippet,
                score: result.score,
            });
        }

        if chunks.is_empty() {
            return Err(RetrieverError::NoResultsAboveThreshold);
        }

        Ok(chunks)
    }

    fn extract_metadata(
        &self,
        payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    ) -> RetrieverResult<ChunkMetadata> {
        let get_string = |key: &str| -> RetrieverResult<String> {
            payload
                .get(key)
                .and_then(|v| match &v.kind {
                    Kind::Keyword(k) => Some(k.clone()),
                    Kind::StringValue(s) => Some(s.clone()),
                    _ => None,
                })
                .ok_or_else(|| RetrieverError::InvalidPayload(format!("Missing or invalid {key}")))
        };

        Ok(ChunkMetadata {
            doc_id: get_string("doc_id")?,
            path: get_string("path")?,
            title_path: get_string("title_path")?,
            section: get_string("section")?,
        })
    }

    fn extract_text(
        &self,
        payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    ) -> RetrieverResult<String> {
        payload
            .get("text")
            .and_then(|v| match &v.kind {
                Kind::StringValue(s) => Some(s.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                RetrieverError::InvalidPayload("Missing or invalid 'text' field".to_string())
            })
    }

    pub async fn ensure_collection_exists(&self) -> RetrieverResult<()> {
        use qdrant_client::qdrant::{
            CreateCollection, Distance, OptimizersConfigDiff, VectorParams, VectorsConfig,
        };

        let collections = self.client.list_collections().await?;

        let exists = collections
            .result
            .into_iter()
            .any(|c| c.name == COLLECTION_NAME);

        if exists {
            return Ok(());
        }

        self.client
            .create_collection(&CreateCollection {
                collection_name: COLLECTION_NAME.into(),
                vectors_config: Some(VectorsConfig::From(VectorParams {
                    size: 1536,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
                optimizers_config: Some(OptimizersConfigDiff {
                    indexing_threshold: Some(20000),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await?;

        tracing::info!(collection = COLLECTION_NAME, "created collection");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_threshold_constant() {
        assert_eq!(SCORE_THRESHOLD, 0.3);
    }

    #[test]
    fn test_default_top_k_constant() {
        assert_eq!(DEFAULT_TOP_K, 6);
    }

    #[test]
    fn test_collection_name_constant() {
        assert_eq!(COLLECTION_NAME, "knowledge_chunks");
    }
}
