use crate::{
    api::error_code::ErrorCode,
    indexer::{IndexerError, IndexResult, MarkdownIndexer},
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 索引任务状态
#[derive(Debug, Clone, Serialize)]
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
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            current_job: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取当前任务信息
    pub async fn get_current_job(&self) -> Option<JobInfo> {
        self.current_job.read().await.as_ref().cloned()
    }

    /// 开始新任务
    async fn start_job(&self) -> Result<String, ReindexError> {
        let mut job = self.current_job.write().await;

        if job.is_some() {
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
        let mut job = self.current_job.write().await;

        if let Some(info) = job.as_mut() {
            info.status = JobStatus::Completed;
            info.ended_at = Some(chrono::Utc::now().to_rfc3339());
            info.result = Some(result);
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

/// POST /api/reindex 请求
#[derive(Debug, Deserialize)]
pub struct ReindexRequest {}

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
    manager: State<JobManager>,
    indexer: State<MarkdownIndexer>,
    _req: Json<ReindexRequest>,
) -> Result<Json<ReindexResponse>, ReindexError> {
    tracing::info!("received reindex request");

    // Start job
    let job_id = manager.start_job().await?;
    let manager_clone = manager.clone();

    // Run indexing in background
    tokio::spawn(async move {
        tracing::info!(job_id = %job_id, "started reindex job");

        match indexer.index().await {
            Ok(result) => {
                tracing::info!(
                    job_id = %job_id,
                    successful = result.successful_chunks,
                    failed = result.failed_chunks,
                    "reindex job completed"
                );
                manager_clone.complete_job(result).await;
            }
            Err(e) => {
                tracing::error!(job_id = %job_id, error = %e, "reindex job failed");
                manager_clone.fail_job(e.to_string()).await;
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
    manager: State<JobManager>,
) -> Json<ReindexStatusResponse> {
    let job = manager.get_current_job().await;

    Json(ReindexStatusResponse { job })
}
