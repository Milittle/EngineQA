pub mod error_code;
pub mod error_mapping;
pub mod feedback;
pub mod query;
pub mod reindex;
pub mod status;

use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::config::AppConfig;

pub fn router<S>(_config: &AppConfig) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    use crate::api;
    use crate::config::AppConfig;

    #[tokio::test]
    async fn health_returns_200() {
        let vars = HashMap::from([
            (
                "INTERNAL_API_BASE_URL".to_string(),
                "https://internal-api.example.com".to_string(),
            ),
            ("INTERNAL_API_TOKEN".to_string(), "token-value".to_string()),
        ]);
        let config = AppConfig::from_map(&vars).expect("config should load");
        let app = api::router::<()>(&config);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .method("GET")
                    .body(Body::empty())
                    .expect("request should be built"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
