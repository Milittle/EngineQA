use crate::{
    config::InternalApiConfig,
    provider::{InferenceProvider, InternalApiProvider, ProviderError},
    vector_store::{StoredChunk, VectorStore, VectorStoreError},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tracing::{debug, error, info, warn};

const DEFAULT_CHUNK_SIZE: usize = 1000; // 中文字符数
const DEFAULT_OVERLAP: usize = 125; // 中文字符数

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone)]
struct Chunk {
    pub doc_id: String,
    pub chunk_id: String,
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

    #[error("Vector store error: {0}")]
    VectorStoreError(#[from] VectorStoreError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Markdown parsing error: {0}")]
    ParseError(String),
}

pub type IndexerResult<T> = Result<T, IndexerError>;

pub struct MarkdownIndexer {
    provider: InternalApiProvider,
    vector_store: Arc<dyn VectorStore>,
    knowledge_dir: PathBuf,
    chunk_size: usize,
    overlap: usize,
}

impl MarkdownIndexer {
    pub fn new(
        config: InternalApiConfig,
        vector_store: Arc<dyn VectorStore>,
        knowledge_dir: &str,
    ) -> IndexerResult<Self> {
        let provider = InternalApiProvider::new(config.clone());
        let knowledge_dir = PathBuf::from(knowledge_dir);

        Ok(Self {
            provider,
            vector_store,
            knowledge_dir,
            chunk_size: DEFAULT_CHUNK_SIZE,
            overlap: DEFAULT_OVERLAP,
        })
    }

    pub async fn index(&self, full_rebuild: bool) -> IndexerResult<IndexResult> {
        let start = Instant::now();

        info!(full_rebuild, "starting markdown indexing");

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

        // Step 2: Inspect existing metadata from vector store
        let existing_files = self.vector_store.list_doc_hashes().await?;

        // Step 3: Determine which files need processing
        let files_to_index = if full_rebuild {
            md_files.clone()
        } else {
            self.determine_files_to_index(&md_files, &existing_files)
        };
        let indexed_files = files_to_index.len();

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

        for (path, hash) in files_to_index {
            match self.process_file(&path, &hash).await {
                Ok((successful, failed, total)) => {
                    total_chunks += total;
                    successful_chunks += successful;
                    failed_chunks += failed;
                }
                Err(e) => {
                    error!(file = %path.to_string_lossy(), error = %e, "failed to process file");
                    failed_files += 1;
                }
            }
        }

        // Step 5: Delete chunks from files that no longer exist
        let deleted_chunks = self
            .delete_obsolete_chunks(existing_files.keys().cloned().collect(), &md_files)
            .await?;

        let duration_ms = start.elapsed().as_millis();

        info!(
            total_files = md_files.len(),
            indexed_files,
            successful_chunks,
            failed_chunks,
            deleted_chunks,
            duration_ms = duration_ms,
            "indexing complete"
        );

        Ok(IndexResult {
            total_files: md_files.len(),
            indexed_files,
            skipped_files: md_files.len() - indexed_files,
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
            warn!(
                "knowledge directory does not exist: {:?}",
                self.knowledge_dir
            );
            return Ok(files);
        }

        let entries = fs::read_dir(&self.knowledge_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                let content = fs::read_to_string(&path)?;
                let hash = compute_hash(&content);
                files.push((path, hash));
            }
        }

        files.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(files)
    }

    fn determine_files_to_index(
        &self,
        md_files: &[(PathBuf, String)],
        existing_files: &HashMap<String, String>,
    ) -> Vec<(PathBuf, String)> {
        let mut files_to_index = Vec::new();

        for (path, hash) in md_files {
            let relative_path = path
                .strip_prefix(&self.knowledge_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let doc_id = compute_doc_id(&relative_path);

            if let Some(existing_hash) = existing_files.get(&doc_id) {
                if existing_hash != hash {
                    debug!(file = %relative_path, "file changed, re-indexing");
                    files_to_index.push((path.clone(), hash.clone()));
                } else {
                    debug!(file = %relative_path, "file unchanged, skipping");
                }
            } else {
                debug!(file = %relative_path, "new file, indexing");
                files_to_index.push((path.clone(), hash.clone()));
            }
        }

        files_to_index
    }

    async fn process_file(&self, path: &Path, _hash: &str) -> IndexerResult<(usize, usize, usize)> {
        let content = fs::read_to_string(path)?;
        let relative_path = path
            .strip_prefix(&self.knowledge_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let doc_id = compute_doc_id(&relative_path);

        // First, delete all existing chunks for this file
        self.vector_store.delete_by_doc_id(&doc_id).await?;

        // Parse and chunk the markdown
        let chunks = self.parse_and_chunk(&content, &relative_path, &doc_id)?;
        let total_chunks = chunks.len();

        // Embed then batch-upsert
        let mut records = Vec::new();
        let mut failed = 0usize;

        for chunk in chunks {
            match self.provider.embed(&chunk.text).await {
                Ok(vector) => {
                    let point_id = format!("{}|{}|{}", chunk.doc_id, chunk.chunk_id, chunk.hash);
                    records.push(StoredChunk {
                        point_id,
                        doc_id: chunk.doc_id,
                        chunk_id: chunk.chunk_id,
                        path: chunk.path,
                        title_path: chunk.title_path,
                        section: chunk.section,
                        text: chunk.text,
                        hash: chunk.hash,
                        vector,
                    });
                }
                Err(e) => {
                    error!(doc_id = %doc_id, error = %e, "failed to embed chunk");
                    failed += 1;
                }
            }
        }

        if !records.is_empty() {
            self.vector_store.upsert_chunks(records).await?;
        }

        let successful = total_chunks - failed;
        Ok((successful, failed, total_chunks))
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

            if trimmed.starts_with('#') {
                if !buffer.trim().is_empty() {
                    let text = buffer.trim().to_string();
                    let hash = compute_hash(&text);
                    chunks.push(Chunk {
                        doc_id: doc_id.to_string(),
                        chunk_id: String::new(),
                        path: path.to_string(),
                        title_path: current_title_path.clone(),
                        section: current_section.clone(),
                        text,
                        hash,
                    });
                }

                buffer.clear();

                let heading_level = trimmed.chars().take_while(|&c| c == '#').count();
                let heading_text = trimmed[heading_level..].trim().to_string();

                current_section = heading_text.clone();

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

        if !buffer.trim().is_empty() {
            let text = buffer.trim().to_string();
            let hash = compute_hash(&text);
            chunks.push(Chunk {
                doc_id: doc_id.to_string(),
                chunk_id: String::new(),
                path: path.to_string(),
                title_path: current_title_path,
                section: current_section,
                text,
                hash,
            });
        }

        self.create_overlapping_chunks(chunks)
    }

    fn create_overlapping_chunks(&self, chunks: Vec<Chunk>) -> IndexerResult<Vec<Chunk>> {
        let mut result = Vec::new();

        for chunk in chunks {
            let chars: Vec<char> = chunk.text.chars().collect();
            let total_chars = chars.len();

            if total_chars <= self.chunk_size {
                result.push(Chunk {
                    chunk_id: format!("{}_chunk_0", chunk.doc_id),
                    ..chunk
                });
                continue;
            }

            let mut start = 0;
            let mut chunk_num = 0;

            while start < total_chars {
                let end = (start + self.chunk_size).min(total_chars);
                let chunk_text: String = chars[start..end].iter().collect();
                let hash = compute_hash(&chunk_text);

                result.push(Chunk {
                    doc_id: chunk.doc_id.clone(),
                    chunk_id: format!("{}_chunk_{}", chunk.doc_id, chunk_num),
                    path: chunk.path.clone(),
                    title_path: chunk.title_path.clone(),
                    section: chunk.section.clone(),
                    text: chunk_text,
                    hash,
                });

                if end == total_chars {
                    break;
                }

                start += self.chunk_size - self.overlap;
                chunk_num += 1;
            }
        }

        Ok(result)
    }

    async fn delete_obsolete_chunks(
        &self,
        existing_doc_ids: HashSet<String>,
        current_files: &[(PathBuf, String)],
    ) -> IndexerResult<usize> {
        let current_doc_ids: HashSet<String> = current_files
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

        let mut deleted_count = 0usize;

        for doc_id in existing_doc_ids {
            if !current_doc_ids.contains(&doc_id) {
                self.vector_store.delete_by_doc_id(&doc_id).await?;
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
