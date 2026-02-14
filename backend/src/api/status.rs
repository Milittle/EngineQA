use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Serialize, Serializer};
use std::sync::Arc;

use crate::{AppState, vector_store::VectorStoreError};

/// 上游健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpstreamHealth {
    /// 健康
    Ok,
    /// 降级
    Degraded,
    /// 不可用
    Unavailable,
}

/// 限流状态
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitState {
    pub rpm_limit: u32,
    pub current_rpm: u32,
}

/// 系统状态响应
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// 推理提供方
    pub provider: String,
    /// 使用的模型
    pub model: String,
    /// 向量存储类型
    pub vector_store: String,
    /// 向量表名
    pub vector_table: String,
    /// 索引大小（文档片段数）
    pub index_size: usize,
    /// 最后索引时间
    #[serde(serialize_with = "serialize_option_datetime")]
    pub last_index_time: Option<String>,
    /// 上游健康状态
    pub upstream_health: UpstreamHealth,
    /// 限流状态
    pub rate_limit_state: RateLimitState,
    /// 向量存储连接状态
    pub vector_store_connected: bool,
    /// 兼容字段：后续版本将移除
    pub qdrant_connected: bool,
}

/// 序列化可选的日期时间
fn serialize_option_datetime<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(v),
        None => serializer.serialize_none(),
    }
}

/// 获取向量表信息
async fn get_collection_info(state: &AppState) -> Result<CollectionInfo, StatusError> {
    state.vector_store.ensure_ready().await?;
    let points_count = state.vector_store.count().await?;

    Ok(CollectionInfo { points_count })
}

#[derive(Debug, Clone)]
struct CollectionInfo {
    points_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("Vector store error: {0}")]
    VectorStoreError(#[from] VectorStoreError),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for StatusError {
    fn into_response(self) -> Response {
        match self {
            StatusError::VectorStoreError(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response()
            }
            StatusError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}

/// 处理 /api/status GET 请求
pub async fn handle_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, StatusError> {
    let collection_info = get_collection_info(&state).await?;
    let last_index_time = state.job_manager.get_last_index_time().await;

    // 简化的上游健康检查 - 在实际生产中应该有更复杂的健康检查逻辑
    let upstream_health = UpstreamHealth::Ok;
    let vector_store_connected = true;

    Ok(Json(StatusResponse {
        provider: state.config.infer_provider.clone(),
        model: state.config.internal_api.chat_model.clone(),
        vector_store: state.config.vector_store.clone(),
        vector_table: state.config.lancedb_table.clone(),
        index_size: collection_info.points_count,
        last_index_time,
        upstream_health,
        rate_limit_state: RateLimitState {
            rpm_limit: state.config.internal_api.chat_rate_limit_rpm,
            current_rpm: 0, // TODO: 从实际的速率限制器中读取
        },
        vector_store_connected,
        qdrant_connected: vector_store_connected,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upstream_health_serialization() {
        let health = UpstreamHealth::Ok;
        let json = serde_json::to_string(&health).unwrap();
        assert_eq!(json, "\"ok\"");
    }
}
