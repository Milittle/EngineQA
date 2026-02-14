use crate::{
    AppState,
    indexer::{IndexResult, IndexerError},
};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 索引任务状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// 任务运行中
    Running,
    /// 任务完成
    Completed,
    /// 任务失败
    Failed,
}

/// 索引任务信息
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    /// 任务 ID
    pub job_id: String,
    /// 任务状态
    pub status: JobStatus,
    /// 开始时间
    pub started_at: String,
    /// 结束时间
    pub ended_at: Option<String>,
    /// 索引结果
    pub result: Option<IndexResult>,
    /// 错误信息
    pub error: Option<String>,
}

/// 索引任务管理器
#[derive(Clone)]
pub struct JobManager {
    current_job: Arc<RwLock<Option<JobInfo>>>,
    last_index_time: Arc<RwLock<Option<String>>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            current_job: Arc::new(RwLock::new(None)),
            last_index_time: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取当前任务信息
    pub async fn get_current_job(&self) -> Option<JobInfo> {
        self.current_job.read().await.as_ref().cloned()
    }

    /// 获取最近一次成功索引时间
    pub async fn get_last_index_time(&self) -> Option<String> {
        self.last_index_time.read().await.clone()
    }

    /// 开始新任务
    async fn start_job(&self) -> Result<String, ReindexError> {
        let mut job = self.current_job.write().await;

        if matches!(
            job.as_ref(),
            Some(JobInfo {
                status: JobStatus::Running,
                ..
            })
        ) {
            return Err(ReindexError::JobInProgress);
        }

        let job_id = Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();

        *job = Some(JobInfo {
            job_id: job_id.clone(),
            status: JobStatus::Running,
            started_at,
            ended_at: None,
            result: None,
            error: None,
        });

        Ok(job_id)
    }

    /// 完成任务
    async fn complete_job(&self, result: IndexResult) {
        let ended_at = chrono::Utc::now().to_rfc3339();
        let mut job = self.current_job.write().await;

        if let Some(info) = job.as_mut() {
            info.status = JobStatus::Completed;
            info.ended_at = Some(ended_at.clone());
            info.result = Some(result);
            info.error = None;
            *self.last_index_time.write().await = Some(ended_at);
        }
    }

    /// 任务失败
    async fn fail_job(&self, error: String) {
        let mut job = self.current_job.write().await;

        if let Some(info) = job.as_mut() {
            info.status = JobStatus::Failed;
            info.ended_at = Some(chrono::Utc::now().to_rfc3339());
            info.error = Some(error);
        }
    }

    /// 清除任务
    pub async fn clear_job(&self) {
        *self.current_job.write().await = None;
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReindexError {
    #[error("Another reindex job is already in progress")]
    JobInProgress,

    #[error("Indexer error: {0}")]
    IndexerError(#[from] IndexerError),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for ReindexError {
    fn into_response(self) -> Response {
        match self {
            ReindexError::JobInProgress => (StatusCode::CONFLICT, self.to_string()).into_response(),
            ReindexError::IndexerError(_) | ReindexError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}

/// POST /api/reindex 请求
#[derive(Debug, Deserialize)]
pub struct ReindexRequest {
    #[serde(default = "default_full_rebuild")]
    pub full: bool,
}

fn default_full_rebuild() -> bool {
    true
}

/// POST /api/reindex 响应
#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub job_id: String,
    pub message: String,
}

/// GET /api/reindex 状态响应
#[derive(Debug, Serialize)]
pub struct ReindexStatusResponse {
    pub job: Option<JobInfo>,
}

/// 处理 /api/reindex POST 请求
pub async fn handle_reindex(
    State(state): State<Arc<AppState>>,
    req: Json<ReindexRequest>,
) -> Result<Json<ReindexResponse>, ReindexError> {
    tracing::info!(full_rebuild = req.full, "received reindex request");

    // Start job
    let job_id = state.job_manager.start_job().await?;
    let job_id_for_task = job_id.clone();
    let state_clone = state.clone();
    let full_rebuild = req.full;

    // Run indexing in background
    tokio::spawn(async move {
        tracing::info!(job_id = %job_id_for_task, "started reindex job");

        match state_clone.indexer.index(full_rebuild).await {
            Ok(result) => {
                tracing::info!(
                    job_id = %job_id_for_task,
                    successful = result.successful_chunks,
                    failed = result.failed_chunks,
                    "reindex job completed"
                );
                state_clone.job_manager.complete_job(result).await;
            }
            Err(e) => {
                tracing::error!(job_id = %job_id_for_task, error = %e, "reindex job failed");
                state_clone.job_manager.fail_job(e.to_string()).await;
            }
        }
    });

    Ok(Json(ReindexResponse {
        job_id,
        message: "Reindex job started successfully".to_string(),
    }))
}

/// 处理 /api/reindex GET 请求
pub async fn handle_reindex_status(
    State(state): State<Arc<AppState>>,
) -> Json<ReindexStatusResponse> {
    let job = state.job_manager.get_current_job().await;

    Json(ReindexStatusResponse { job })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_serializes_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&JobStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&JobStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&JobStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[tokio::test]
    async fn start_job_allows_new_job_after_completion() {
        let manager = JobManager::new();
        let first_job_id = manager.start_job().await.expect("first job should start");
        assert!(!first_job_id.is_empty());

        manager
            .complete_job(IndexResult {
                total_files: 1,
                indexed_files: 1,
                skipped_files: 0,
                failed_files: 0,
                total_chunks: 1,
                successful_chunks: 1,
                failed_chunks: 0,
                deleted_chunks: 0,
                duration_ms: 10,
            })
            .await;

        let second_job_id = manager
            .start_job()
            .await
            .expect("completed job should not block new job");
        assert_ne!(first_job_id, second_job_id);
        assert!(manager.get_last_index_time().await.is_some());
    }
}
