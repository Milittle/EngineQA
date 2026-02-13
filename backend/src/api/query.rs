use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    api::{error_code::ErrorCode, error_mapping},
    provider::{ChatMessage, InferenceProvider},
    rag::RetrievedChunk,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub question: String,
    #[serde(default = "default_top_k")]
    pub top_k: u64,
}

fn default_top_k() -> u64 {
    6
}

#[derive(Debug, Serialize)]
pub struct QuerySource {
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f32,
}

impl From<RetrievedChunk> for QuerySource {
    fn from(chunk: RetrievedChunk) -> Self {
        Self {
            title: chunk.metadata.title_path.clone(),
            path: chunk.metadata.path,
            snippet: chunk.snippet,
            score: chunk.score,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<QuerySource>,
    pub degraded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub trace_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Provider error: {0}")]
    ProviderError(#[from] crate::provider::ProviderError),
    #[error("Retrieval error: {0}")]
    RetrievalError(#[from] crate::rag::RetrieverError),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type QueryResult<T> = Result<T, QueryError>;

impl IntoResponse for QueryError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

const CHAT_TEMPERATURE: f32 = 0.2;
const MAX_TOKENS: u32 = 512;

const SYSTEM_PROMPT: &str = r#"
你是一个广告引擎维优专家的智能助手。

## 重要规则

1. **仅基于提供的参考资料回答问题**
   - 如果参考资料中没有足够的信息，请明确说明"根据现有资料，我不确定"
   - **绝对不要编造或推测答案**

2. **提供可操作的排查建议**
   - 针对故障问题，给出步骤化的排查建议
   - 每个建议应基于参考资料中的实际内容

3. **答案结构清晰**
   - 直接回答问题
   - 如有多个解决方案，分别说明
   - 引用来源时要准确

4. **语言风格**
   - 使用专业但易懂的中文
   - 避免冗长，保持简洁
   - 技术术语保持一致

## 回答格式

根据参考资料，问题的答案是：
[答案内容]

相关参考：
- [来源1的标题]
- [来源2的标题]
"#;

pub async fn handle_query(
    State(state): State<Arc<AppState>>,
    req: Json<QueryRequest>,
) -> QueryResult<Json<QueryResponse>> {
    let trace_id = Uuid::new_v4().to_string();
    let question = &req.question;

    tracing::info!(
        trace_id = %trace_id,
        question = %question,
        top_k = req.top_k,
        "received query request"
    );

    // Step 1: Embed query
    let query_vector = match state.provider.embed(question).await {
        Ok(vec) => vec,
        Err(e) => {
            let error_code = error_mapping::map_provider_error(&e);
            tracing::warn!(
                trace_id = %trace_id,
                error_code = %error_code,
                error = %e,
                "embedding failed"
            );
            return Ok(build_degraded_response(&trace_id, error_code, vec![]));
        }
    };

    // Step 2: Retrieve relevant chunks
    let retrieved_chunks = state.retriever.retrieve(query_vector, Some(req.top_k)).await;

    let chunks = match retrieved_chunks {
        Ok(chunks) if !chunks.is_empty() => chunks,
        Ok(_) => return Ok(build_no_match_response(&trace_id)),
        Err(e) => {
            tracing::warn!(
                trace_id = %trace_id,
                error = %e,
                "retrieval failed"
            );
            return Ok(build_degraded_response(
                &trace_id,
                ErrorCode::RetrievalFailed,
                vec![],
            ));
        }
    };

    // Step 3: Build context from chunks
    let context = build_context(&chunks);

    // Step 4: Generate answer using chat
    let messages = build_messages(question, &context);

    let answer = match state
        .provider
        .chat(messages, CHAT_TEMPERATURE, MAX_TOKENS)
        .await
    {
        Ok(answer) => answer,
        Err(e) => {
            let error_code = error_mapping::map_provider_error(&e);
            tracing::error!(
                trace_id = %trace_id,
                error_code = %error_code,
                error = %e,
                "chat generation failed"
            );

            // 如果是上游错误且应该降级，返回检索到的片段
            let sources: Vec<QuerySource> =
                chunks.into_iter().map(QuerySource::from).collect();

            if error_mapping::should_degrade(error_code) {
                return Ok(build_degraded_with_sources_response(
                    &trace_id,
                    error_code,
                    sources,
                ));
            } else {
                return Ok(build_degraded_response(&trace_id, error_code, vec![]));
            }
        }
    };

    tracing::info!(
        trace_id = %trace_id,
        sources_count = chunks.len(),
        "query completed successfully"
    );

    Ok(Json(QueryResponse {
        answer,
        sources: chunks.into_iter().map(QuerySource::from).collect(),
        degraded: false,
        error_code: None,
        trace_id,
    }))
}

fn build_context(chunks: &[RetrievedChunk]) -> String {
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            format!(
                "[来源{}] {}\n路径: {}\n内容: {}\n",
                i + 1,
                chunk.metadata.title_path,
                chunk.metadata.path,
                chunk.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_messages(question: &str, context: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!(
                "问题: {}\n\n参考资料:\n{}",
                question, context
            ),
        },
    ]
}

fn build_no_match_response(trace_id: &str) -> Json<QueryResponse> {
    Json(QueryResponse {
        answer: "根据现有知识库，我没有找到相关的参考资料来回答这个问题。请尝试更具体的问题描述，或者联系技术团队获取更多帮助。".to_string(),
        sources: vec![],
        degraded: true,
        error_code: Some(ErrorCode::NoMatch.to_string()),
        trace_id: trace_id.to_string(),
    })
}

fn build_degraded_response(trace_id: &str, error_code: ErrorCode, sources: Vec<QuerySource>) -> Json<QueryResponse> {
    let description = error_mapping::get_error_description(error_code);

    Json(QueryResponse {
        answer: format!("服务暂时不可用：{}。", description),
        sources,
        degraded: true,
        error_code: Some(error_code.to_string()),
        trace_id: trace_id.to_string(),
    })
}

fn build_degraded_with_sources_response(trace_id: &str, error_code: ErrorCode, sources: Vec<QuerySource>) -> Json<QueryResponse> {
    let description = error_mapping::get_error_description(error_code);
    let sources_text = if sources.is_empty() {
        "没有找到相关的参考文档。".to_string()
    } else {
        format!(
            "以下是一些相关的参考文档，您可以自行查阅：\n{}",
            sources
                .iter()
                .map(|s| format!("- [{}] {}", s.title, s.path))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    Json(QueryResponse {
        answer: format!("AI 生成服务暂时不可用：{}。\n\n{}", description, sources_text),
        sources,
        degraded: true,
        error_code: Some(error_code.to_string()),
        trace_id: trace_id.to_string(),
    })
}
