pub mod error_code;
pub mod error_mapping;
pub mod query;
pub mod reindex;

use axum::{Json, Router, routing::get, routing::post};
use serde::Serialize;

use crate::config::AppConfig;

pub fn router(config: &AppConfig) -> Router {
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
    use tower::ServiceExt;

    use crate::api;
    use crate::config::AppConfig;

    #[tokio::test]
    async fn health_returns_200() {
        let app = api::router(&AppConfig::from_env().expect("Config should load"));
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
