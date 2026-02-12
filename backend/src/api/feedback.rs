use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 反馈评分
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackRating {
    /// 有用
    Useful,
    /// 无用
    Useless,
}

/// 反馈请求
#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    /// 问题
    pub question: String,
    /// 答案
    pub answer: String,
    /// 评分
    pub rating: FeedbackRating,
    /// 备注（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 错误码（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 追踪 ID
    pub trace_id: String,
}

/// 反馈记录
#[derive(Debug, Clone, Serialize)]
pub struct FeedbackRecord {
    /// 记录 ID
    pub id: String,
    /// 问题
    pub question: String,
    /// 答案
    pub answer: String,
    /// 评分
    pub rating: FeedbackRating,
    /// 备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 错误码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 追踪 ID
    pub trace_id: String,
    /// 创建时间
    pub created_at: String,
}

/// 反馈存储（内存存储，生产环境应替换为数据库）
#[derive(Clone)]
pub struct FeedbackStore {
    records: Arc<RwLock<Vec<FeedbackRecord>>>,
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 保存反馈
    pub async fn save(&self, request: FeedbackRequest) -> Result<FeedbackRecord, FeedbackError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        let record = FeedbackRecord {
            id: id.clone(),
            question: request.question.clone(),
            answer: request.answer.clone(),
            rating: request.rating,
            comment: request.comment,
            error_code: request.error_code,
            trace_id: request.trace_id.clone(),
            created_at,
        };

        let mut records = self.records.write().await;
        records.push(record.clone());

        tracing::info!(
            id = %id,
            rating = ?request.rating,
            trace_id = %request.trace_id,
            "feedback saved"
        );

        Ok(record)
    }

    /// 获取所有反馈
    pub async fn get_all(&self) -> Vec<FeedbackRecord> {
        self.records.read().await.clone()
    }

    /// 根据 trace_id 获取反馈
    pub async fn get_by_trace_id(&self, trace_id: &str) -> Option<FeedbackRecord> {
        let records = self.records.read().await;
        records.iter().find(|r| r.trace_id == trace_id).cloned()
    }
}

impl Default for FeedbackStore {
    fn default() -> Self {
        Self::new()
    }
}

/// 反馈响应
#[derive(Debug, Serialize)]
pub struct FeedbackResponse {
    pub ok: bool,
    pub id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FeedbackError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// 处理 /api/feedback POST 请求
pub async fn handle_feedback(
    store: State<FeedbackStore>,
    req: Json<FeedbackRequest>,
) -> Result<Json<FeedbackResponse>, FeedbackError> {
    tracing::info!(
        trace_id = %req.trace_id,
        rating = ?req.rating,
        "received feedback"
    );

    // 验证输入
    if req.question.trim().is_empty() {
        return Err(FeedbackError::InvalidInput("Question cannot be empty".to_string()));
    }

    if req.answer.trim().is_empty() {
        return Err(FeedbackError::InvalidInput("Answer cannot be empty".to_string()));
    }

    if req.trace_id.trim().is_empty() {
        return Err(FeedbackError::InvalidInput("Trace ID cannot be empty".to_string()));
    }

    // 保存反馈
    let record = store.save(req.0).await?;

    Ok(Json(FeedbackResponse {
        ok: true,
        id: record.id,
    }))
}
