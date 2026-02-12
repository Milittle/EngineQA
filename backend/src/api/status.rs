use axum::{extract::State, Json};
use serde::{Serialize, Serializer};
use std::collections::HashMap;

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
    /// 索引大小（文档数）
    pub index_size: usize,
    /// 最后索引时间
    #[serde(serialize_with = "serialize_option_datetime")]
    pub last_index_time: Option<String>,
    /// 上游健康状态
    pub upstream_health: UpstreamHealth,
    /// 限流状态
    pub rate_limit_state: RateLimitState,
    /// Qdrant 连接状态
    pub qdrant_connected: bool,
}

/// 序列化可选的日期时间
fn serialize_option_datetime<S>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(v),
        None => serializer.serialize_none(),
    }
}

/// 获取 Qdrant collection 信息
async fn get_collection_info(qdrant: &qdrant_client::Qdrant) -> Result<CollectionInfo, StatusError> {
    let collection_name = "knowledge_chunks";

    match qdrant.get_collection(collection_name).await {
        Ok(result) => {
            let info = result.result;
            Ok(CollectionInfo {
                points_count: info.points_count as usize,
                indexed_points_count: info.indexed_points_count as usize,
            })
        }
        Err(e) => {
            if e.to_string().contains("Not found") {
                // Collection doesn't exist yet
                Ok(CollectionInfo {
                    points_count: 0,
                    indexed_points_count: 0,
                })
            } else {
                Err(StatusError::QdrantError(e))
            }
        }
    }
}

#[derive(Debug, Clone)]
struct CollectionInfo {
    points_count: usize,
    indexed_points_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("Qdrant error: {0}")]
    QdrantError(#[from] qdrant_client::QdrantError),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// 处理 /api/status GET 请求
pub async fn handle_status(
    config: State<crate::config::AppConfig>,
    qdrant: State<qdrant_client::Qdrant>,
) -> Result<Json<StatusResponse>, StatusError> {
    let collection_info = get_collection_info(&qdrant).await?;

    // 简化的上游健康检查 - 在实际生产中应该有更复杂的健康检查逻辑
    let upstream_health = UpstreamHealth::Ok;

    Ok(Json(StatusResponse {
        provider: config.infer_provider.clone(),
        model: config.internal_api.chat_model.clone(),
        index_size: collection_info.points_count,
        last_index_time: None, // TODO: 从某个持久化存储中读取
        upstream_health,
        rate_limit_state: RateLimitState {
            rpm_limit: config.internal_api.chat_rate_limit_rpm,
            current_rpm: 0, // TODO: 从实际的速率限制器中读取
        },
        qdrant_connected: collection_info.indexed_points_count > 0 || collection_info.points_count == 0,
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
