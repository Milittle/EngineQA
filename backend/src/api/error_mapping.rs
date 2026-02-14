use crate::api::error_code::ErrorCode;
use crate::provider::ProviderError;
use reqwest::StatusCode;

/// 将 ProviderError 映射为业务错误码
pub fn map_provider_error(error: &ProviderError) -> ErrorCode {
    match error {
        ProviderError::RequestError(err) => {
            // 判断是否为超时错误
            if err.is_timeout() || err.is_connect() {
                ErrorCode::UpstreamTimeout
            } else {
                ErrorCode::UpstreamUnavailable
            }
        }
        ProviderError::ApiError { status, message: _ } => map_status_code(*status),
        ProviderError::Timeout => ErrorCode::UpstreamTimeout,
        ProviderError::SerializationError(_) => ErrorCode::InternalError,
    }
}

/// 根据 HTTP 状态码映射错误码
pub fn map_status_code(status: StatusCode) -> ErrorCode {
    match status {
        StatusCode::UNAUTHORIZED => ErrorCode::UpstreamAuth,
        StatusCode::FORBIDDEN => ErrorCode::UpstreamAuth,
        StatusCode::TOO_MANY_REQUESTS => ErrorCode::UpstreamRateLimit,
        StatusCode::REQUEST_TIMEOUT => ErrorCode::UpstreamTimeout,
        StatusCode::GATEWAY_TIMEOUT => ErrorCode::UpstreamTimeout,
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE => ErrorCode::UpstreamUnavailable,
        _ => {
            if status.is_client_error() {
                ErrorCode::UpstreamError
            } else if status.is_server_error() {
                ErrorCode::UpstreamUnavailable
            } else {
                ErrorCode::UpstreamError
            }
        }
    }
}

/// 获取错误码的可读描述
pub fn get_error_description(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::UpstreamTimeout => "上游服务响应超时，请稍后重试",
        ErrorCode::UpstreamRateLimit => "上游服务限流，请求过于频繁",
        ErrorCode::UpstreamAuth => "上游服务认证失败，请检查 API Token",
        ErrorCode::UpstreamUnavailable => "上游服务不可用，请稍后重试",
        ErrorCode::UpstreamError => "上游服务返回错误",
        ErrorCode::RetrievalFailed => "检索服务失败，请检查向量存储连接",
        ErrorCode::NoMatch => "未找到相关资料，请尝试其他问题",
        ErrorCode::InternalError => "内部服务错误，请联系技术团队",
    }
}

/// 是否应该降级（返回检索片段而不是完整答案）
pub fn should_degrade(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::UpstreamTimeout
            | ErrorCode::UpstreamRateLimit
            | ErrorCode::UpstreamAuth
            | ErrorCode::UpstreamUnavailable
            | ErrorCode::UpstreamError
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_status_401() {
        assert_eq!(
            map_status_code(StatusCode::UNAUTHORIZED),
            ErrorCode::UpstreamAuth
        );
    }

    #[test]
    fn test_map_status_429() {
        assert_eq!(
            map_status_code(StatusCode::TOO_MANY_REQUESTS),
            ErrorCode::UpstreamRateLimit
        );
    }

    #[test]
    fn test_map_status_500() {
        assert_eq!(
            map_status_code(StatusCode::INTERNAL_SERVER_ERROR),
            ErrorCode::UpstreamUnavailable
        );
    }

    #[test]
    fn test_map_status_504() {
        assert_eq!(
            map_status_code(StatusCode::GATEWAY_TIMEOUT),
            ErrorCode::UpstreamTimeout
        );
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::UpstreamTimeout.as_str(), "UPSTREAM_TIMEOUT");
        assert_eq!(ErrorCode::UpstreamRateLimit.as_str(), "UPSTREAM_RATE_LIMIT");
    }

    #[test]
    fn test_should_degrade() {
        assert!(should_degrade(ErrorCode::UpstreamTimeout));
        assert!(should_degrade(ErrorCode::UpstreamRateLimit));
        assert!(should_degrade(ErrorCode::UpstreamAuth));
        assert!(should_degrade(ErrorCode::UpstreamUnavailable));
        assert!(!should_degrade(ErrorCode::NoMatch));
        assert!(!should_degrade(ErrorCode::RetrievalFailed));
    }
}
