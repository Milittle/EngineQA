use crate::config::InternalApiConfig;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: StatusCode, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Timeout waiting for upstream")]
    Timeout,
}

pub type ProviderResult<T> = Result<T, ProviderError>;

#[async_trait::async_trait]
pub trait InferenceProvider: Send + Sync {
    async fn embed(&self, text: &str) -> ProviderResult<Vec<f32>>;

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> ProviderResult<String>;
}

pub struct InternalApiProvider {
    config: InternalApiConfig,
    client: Client,
}

impl InternalApiProvider {
    pub fn new(config: InternalApiConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.llm_timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    async fn do_request<R, S>(&self, path: &str, request: &R, timeout_ms: u64) -> ProviderResult<S>
    where
        R: Serialize + ?Sized,
        S: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);
        let trace_id = Uuid::new_v4().to_string();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("X-Request-Id", &trace_id)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_millis(timeout_ms))
            .json(request)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            response.json().await.map_err(ProviderError::from)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            Err(ProviderError::ApiError {
                status,
                message: error_text,
            })
        }
    }

    async fn embed_with_retry(&self, text: &str) -> ProviderResult<Vec<f32>> {
        let max_retries = self.config.retry_embed_max as usize;

        for attempt in 0..=max_retries {
            match self.embed_once(text).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < max_retries => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        error = %e,
                        "embed request failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    async fn chat_with_retry(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> ProviderResult<String> {
        let max_retries = self.config.retry_chat_max as usize;

        for attempt in 0..=max_retries {
            match self
                .chat_once(messages.clone(), temperature, max_tokens)
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) if attempt < max_retries => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        error = %e,
                        "chat request failed, retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    async fn embed_once(&self, text: &str) -> ProviderResult<Vec<f32>> {
        let request = EmbeddingRequest {
            model: self.config.embed_model.clone(),
            input: text.to_string(),
        };

        let response: EmbeddingResponse = self
            .do_request(
                &self.config.embed_path,
                &request,
                self.config.embed_timeout_ms,
            )
            .await?;

        response
            .data
            .first()
            .map(|data| data.embedding.clone())
            .ok_or_else(|| ProviderError::ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                message: "No embedding data returned".to_string(),
            })
    }

    async fn chat_once(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> ProviderResult<String> {
        let request = ChatRequest {
            model: self.config.chat_model.clone(),
            messages,
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
        };

        let response: ChatResponse = self
            .do_request(&self.config.chat_path, &request, self.config.llm_timeout_ms)
            .await?;

        response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| ProviderError::ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                message: "No chat response returned".to_string(),
            })
    }
}

#[async_trait::async_trait]
impl InferenceProvider for InternalApiProvider {
    async fn embed(&self, text: &str) -> ProviderResult<Vec<f32>> {
        self.embed_with_retry(text).await
    }

    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> ProviderResult<String> {
        self.chat_with_retry(messages, temperature, max_tokens)
            .await
    }
}
