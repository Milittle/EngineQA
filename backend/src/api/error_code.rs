use std::fmt;

/// 标准化的业务错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// 上游超时
    UpstreamTimeout,
    /// 上游限流
    UpstreamRateLimit,
    /// 上游认证失败
    UpstreamAuth,
    /// 上游不可用
    UpstreamUnavailable,
    /// 上游返回错误
    UpstreamError,
    /// 检索失败
    RetrievalFailed,
    /// 没有匹配的结果
    NoMatch,
    /// 内部错误
    InternalError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::UpstreamTimeout => "UPSTREAM_TIMEOUT",
            ErrorCode::UpstreamRateLimit => "UPSTREAM_RATE_LIMIT",
            ErrorCode::UpstreamAuth => "UPSTREAM_AUTH",
            ErrorCode::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            ErrorCode::UpstreamError => "UPSTREAM_ERROR",
            ErrorCode::RetrievalFailed => "RETRIEVAL_FAILED",
            ErrorCode::NoMatch => "NO_MATCH",
            ErrorCode::InternalError => "INTERNAL_ERROR",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().to_string()
    }
}
