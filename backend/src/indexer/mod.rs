use crate::{
    config::InternalApiConfig,
    provider::{EmbeddingRequest, InternalApiProvider, ProviderError},
    rag::VectorRetriever,
};
use qdrant_client::{
    qdrant::{Payload, PayloadInterface, PointStruct, Value},
    Qdrant,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

const DEFAULT_CHUNK_SIZE: usize = 1000; // 中文字符数
const DEFAULT_OVERLAP: usize = 125; // 中文字符数
const MAX_CONCURRENT_EMBEDDINGS: usize = 8;

#[derive(Debug, Clone)]
pub struct IndexResult {
    pub total_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub failed_files: usize,
    pub total_chunks: usize,
    pub successful_chunks: usize,
    pub failed_chunks: usize,
    pub deleted_chunks: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMetadata {
    pub doc_id: String,
    pub path: String,
    pub hash: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct Chunk {
    pub doc_id: String,
    pub path: String,
    pub title_path: String,
    pub section: String,
    pub text: String,
    pub hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("Qdrant error: {0}")]
    QdrantError(#[from] qdrant_client::QdrantError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Markdown parsing error: {0}")]
    ParseError(String),
}

pub type IndexerResult<T> = Result<T, IndexerError>;

pub struct MarkdownIndexer {
    provider: InternalApiProvider,
    qdrant: Qdrant,
    knowledge_dir: PathBuf,
    chunk_size: usize,
    overlap: usize,
}

impl MarkdownIndexer {
    pub fn new(
        config: InternalApiConfig,
        qdrant_url: &str,
        knowledge_dir: &str,
    ) -> IndexerResult<Self> {
        let provider = InternalApiProvider::new(config.clone());
        let qdrant = Qdrant::from_url(qdrant_url).build()?;
        let knowledge_dir = PathBuf::from(knowledge_dir);

        Ok(Self {
            provider,
            qdrant,
            knowledge_dir,
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_OVERLAP,
        })
    }

    pub async fn index(&self) -> IndexerResult<IndexResult> {
        let start = Instant::now();

        info!("starting markdown indexing");

        // Step 1: Scan all markdown files
        let md_files = self.scan_markdown_files().await?;

        if md_files.is_empty() {
            warn!("no markdown files found in {:?}", self.knowledge_dir);
            return Ok(IndexResult {
                total_files: 0,
                indexed_files: 0,
                skipped_files: 0,
                failed_files: 0,
                total_chunks: 0,
                successful_chunks: 0,
                failed_chunks: 0,
                deleted_chunks: 0,
                duration_ms: start.elapsed().as_millis(),
            });
        }

        // Step 2: Get existing file metadata from Qdrant
        let existing_files = self.get_existing_files().await?;

        // Step 3: Determine which files need processing
        let files_to_index = self.determine_files_to_index(&md_files, &existing_files)?;

        info!(
            total_files = md_files.len(),
            files_to_index = files_to_index.len(),
            "files analysis complete"
        );

        // Step 4: Process each file
        let mut total_chunks = 0usize;
        let mut successful_chunks = 0usize;
        let mut failed_chunks = 0usize;
        let mut failed_files = 0usize;

        let semaphore = Semaphore::new(MAX_CONCURRENT_EMBEDDINGS);

        for file_result in files_to_index {
            match file_result {
                Ok((path, hash)) => {
                    let permit = semaphore.clone().acquire_owned().await?;
                    match self.process_file(&path, &hash).await {
                        Ok(chunks_count) => {
                            total_chunks += chunks_count.0 + chunks_count.1;
                            successful_chunks += chunks_count.0;
                            failed_chunks += chunks_count.1;
                        }
                        Err(e) => {
                            error!(file = %path.to_string_lossy(), error = %e, "failed to process file");
                            failed_files += 1;
                        }
                    }
                    drop(permit);
                }
                Err(e) => {
                    error!(error = %e, "failed to determine file status");
                }
            }
        }

        // Step 5: Delete chunks from files that no longer exist
        let deleted_chunks = self.delete_obsolete_chunks(&md_files).await?;

        let duration_ms = start.elapsed().as_millis();

        info!(
            total_files = md_files.len(),
            indexed_files = files_to_index.len(),
            successful_chunks,
            failed_chunks,
            deleted_chunks,
            duration_ms = duration_ms,
            "indexing complete"
        );

        Ok(IndexResult {
            total_files: md_files.len(),
            indexed_files: files_to_index.len(),
            skipped_files: md_files.len() - files_to_index.len(),
            failed_files,
            total_chunks,
            successful_chunks,
            failed_chunks,
            deleted_chunks,
            duration_ms,
        })
    }

    async fn scan_markdown_files(&self) -> IndexerResult<Vec<(PathBuf, String)>> {
        let mut files = Vec::new();

        if !self.knowledge_dir.exists() {
            warn!("knowledge directory does not exist: {:?}", self.knowledge_dir);
            return Ok(files);
        }

        let entries = fs::read_dir(&self.knowledge_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                let content = fs::read_to_string(&path)?;
                let hash = compute_hash(&content);
                files.push((path, hash));
            }
        }

        files.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(files)
    }

    async fn get_existing_files(&self) -> IndexerResult<HashMap<String, FileMetadata>> {
        let collection_name = "knowledge_chunks";

        let scroll_result = self
            .qdrant
            .scroll(
                collection_name,
                None,
                10000,
                None,
                None,
                None,
                Some(vec!["doc_id".to_string(), "path".to_string(), "hash".to_string()]),
            )
            .await?;

        let mut files_map = HashMap::new();

        for point in scroll_result.result.points {
            if let Some(doc_id) = extract_payload_string(&point.payload, "doc_id") {
                if let Some(path) = extract_payload_string(&point.payload, "path") {
                    if let Some(hash) = extract_payload_string(&point.payload, "hash") {
                        let metadata = FileMetadata {
                            doc_id,
                            path,
                            hash,
                            updated_at: chrono::Utc::now(),
                        };
                        files_map.insert(metadata.doc_id.clone(), metadata);
                    }
                }
            }
        }

        Ok(files_map)
    }

    fn determine_files_to_index(
        &self,
        md_files: &[(PathBuf, String)],
        existing_files: &HashMap<String, FileMetadata>,
    ) -> IndexerResult<Vec<Result<(PathBuf, String), IndexerError>>> {
        let mut files_to_index = Vec::new();

        for (path, hash) in md_files {
            let relative_path = path
                .strip_prefix(&self.knowledge_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let doc_id = compute_doc_id(&relative_path);

            if let Some(existing) = existing_files.get(&doc_id) {
                if existing.hash != *hash {
                    debug!(file = %relative_path, "file changed, re-indexing");
                    files_to_index.push(Ok((path.clone(), hash.clone())));
                } else {
                    debug!(file = %relative_path, "file unchanged, skipping");
                }
            } else {
                debug!(file = %relative_path, "new file, indexing");
                files_to_index.push(Ok((path.clone(), hash.clone())));
            }
        }

        Ok(files_to_index)
    }

    async fn process_file(&self, path: &Path, hash: &str) -> IndexerResult<(usize, usize)> {
        let content = fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(&self.knowledge_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let doc_id = compute_doc_id(&relative_path);

        // First, delete all existing chunks for this file
        self.delete_file_chunks(&doc_id).await?;

        // Parse and chunk the markdown
        let chunks = self.parse_and_chunk(&content, &relative_path, &doc_id)?;

        // Embed and upsert chunks
        let mut successful = 0;
        let mut failed = 0;

        for chunk in chunks {
            match self.embed_and_upsert(chunk).await {
                Ok(_) => successful += 1,
                Err(e) => {
                    error!(doc_id = %chunk.doc_id, error = %e, "failed to embed chunk");
                    failed += 1;
                }
            }
        }

        Ok((successful, failed))
    }

    fn parse_and_chunk(
        &self,
        content: &str,
        path: &str,
        doc_id: &str,
    ) -> IndexerResult<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut current_title_path = String::new();
        let mut current_section = String::new();
        let mut buffer = String::new();

        for line in &lines {
            let trimmed = line.trim();

            // Check for headings
            if trimmed.starts_with('#') {
                // Save previous chunk if buffer has content
                if !buffer.trim().is_empty() {
                    let text = buffer.trim().to_string();
                    if !text.is_empty() {
                        chunks.push(Chunk {
                            doc_id: doc_id.to_string(),
                            path: path.to_string(),
                            title_path: current_title_path.clone(),
                            section: current_section.clone(),
                            text,
                            hash: compute_hash(&text),
                        });
                    }
                }

                buffer.clear();

                // Parse heading level
                let heading_level = trimmed.chars().take_while(|&c| c == '#').count();
                let heading_text = trimmed[heading_level..].trim().to_string();

                current_section = heading_text.clone();

                // Update title path based on heading level
                if heading_level == 1 {
                    current_title_path = heading_text;
                } else if heading_level == 2 {
                    if !current_title_path.is_empty() {
                        current_title_path.push_str(" / ");
                    }
                    current_title_path.push_str(&heading_text);
                } else if heading_level == 3 {
                    current_title_path.push_str(" > ");
                    current_title_path.push_str(&heading_text);
                }
            } else {
                buffer.push_str(line);
                buffer.push('\n');
            }
        }

        // Don't forget the last chunk
        if !buffer.trim().is_empty() {
            let text = buffer.trim().to_string();
            if !text.is_empty() {
                chunks.push(Chunk {
                    doc_id: doc_id.to_string(),
                    path: path.to_string(),
                    title_path: current_title_path,
                    section: current_section,
                    text,
                    hash: compute_hash(&text),
                });
            }
        }

        // Now split into overlapping chunks
        let overlapped_chunks = self.create_overlapping_chunks(chunks)?;

        Ok(overlapped_chunks)
    }

    fn create_overlapping_chunks(&self, chunks: Vec<Chunk>) -> IndexerResult<Vec<Chunk>> {
        let mut result = Vec::new();

        for chunk in chunks {
            let chars: Vec<char> = chunk.text.chars().collect();
            let total_chars = chars.len();

            if total_chars <= self.chunk_size {
                result.push(chunk);
                continue;
            }

            // Create overlapping chunks
            let mut start = 0;
            let mut chunk_num = 0;

            while start < total_chars {
                let end = (start + self.chunk_size).min(total_chars);
                let chunk_text: String = chars[start..end].iter().collect();

                result.push(Chunk {
                    doc_id: format!("{}_chunk_{}", chunk.doc_id, chunk_num),
                    path: chunk.path.clone(),
                    title_path: chunk.title_path.clone(),
                    section: chunk.section.clone(),
                    text: chunk_text,
                    hash: compute_hash(&chunk_text),
                });

                start += self.chunk_size - self.overlap;
                chunk_num += 1;
            }
        }

        Ok(result)
    }

    async fn embed_and_upsert(&self, chunk: Chunk) -> IndexerResult<()> {
        let vector = self.provider.embed(&chunk.text).await?;

        let point_id = format!("{}|{}", chunk.doc_id, chunk.hash);

        let mut payload = HashMap::new();
        payload.insert("doc_id".to_string(), Value::from(chunk.doc_id));
        payload.insert("path".to_string(), Value::from(chunk.path));
        payload.insert("title_path".to_string(), Value::from(chunk.title_path));
        payload.insert("section".to_string(), Value::from(chunk.section));
        payload.insert("text".to_string(), Value::from(chunk.text));
        payload.insert("hash".to_string(), Value::from(chunk.hash));

        let point = PointStruct::new(
            qdrant_client::qdrant::PointId::from(point_id),
            vector,
            Payload::from(payload),
        );

        self.qdrant
            .upsert_points_blocking(
                "knowledge_chunks",
                None,
                vec![point],
                None,
            )
            .await?;

        Ok(())
    }

    async fn delete_file_chunks(&self, doc_id: &str) -> IndexerResult<()> {
        let filter = qdrant_client::qdrant::Filter::must([
            qdrant_client::qdrant::Condition::matches(
                "doc_id",
                qdrant_client::qdrant::MatchValue::Keyword(doc_id.to_string()),
            ),
        ]);

        self.qdrant
            .delete_points("knowledge_chunks", None, filter, None)
            .await?;

        Ok(())
    }

    async fn delete_obsolete_chunks(&self, current_files: &[(PathBuf, String)]) -> IndexerResult<u64> {
        let existing_files = self.get_existing_files().await?;

        let current_doc_ids: std::collections::HashSet<String> = current_files
            .iter()
            .map(|(path, _)| {
                let relative_path = path
                    .strip_prefix(&self.knowledge_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                compute_doc_id(&relative_path)
            })
            .collect();

        let mut deleted_count = 0u64;

        for doc_id in existing_files.keys() {
            if !current_doc_ids.contains(doc_id) {
                self.delete_file_chunks(doc_id).await?;
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn compute_doc_id(path: &str) -> String {
    path.replace('/', "_").replace('\\', "_")
}

fn extract_payload_string(
    payload: &HashMap<String, Value>,
    key: &str,
) -> Option<String> {
    payload.get(key).and_then(|v| match v {
        Value::Keyword(k) => Some(k.clone()),
        Value::StringValue(s) => Some(s.clone()),
        _ => None,
    })
}
