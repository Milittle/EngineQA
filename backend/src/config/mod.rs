use std::{collections::HashMap, env, fmt, net::SocketAddr, str::FromStr};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub infer_provider: String,
    pub qdrant_url: String,
    pub knowledge_dir: String,
    pub internal_api: InternalApiConfig,
}

#[derive(Debug, Clone)]
pub struct InternalApiConfig {
    pub base_url: String,
    pub token: String,
    pub chat_path: String,
    pub embed_path: String,
    pub chat_model: String,
    pub embed_model: String,
    pub llm_timeout_ms: u64,
    pub embed_timeout_ms: u64,
    pub outbound_max_concurrency: usize,
    pub chat_rate_limit_rpm: u32,
    pub chat_burst: u32,
    pub retry_chat_max: u32,
    pub retry_embed_max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingEnv(&'static str),
    InvalidEnv {
        key: &'static str,
        value: String,
        reason: &'static str,
    },
    InvalidSocketAddr {
        host: String,
        port: u16,
        error: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingEnv(key) => write!(f, "missing required env: {key}"),
            ConfigError::InvalidEnv { key, value, reason } => {
                write!(f, "invalid env {key}={value}: {reason}")
            }
            ConfigError::InvalidSocketAddr { host, port, error } => {
                write!(f, "invalid socket addr {host}:{port}: {error}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let vars = env::vars().collect::<HashMap<String, String>>();
        Self::from_map(&vars)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        let raw = format!("{}:{}", self.host, self.port);
        SocketAddr::from_str(&raw).map_err(|err| ConfigError::InvalidSocketAddr {
            host: self.host.clone(),
            port: self.port,
            error: err.to_string(),
        })
    }

    pub(crate) fn from_map(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let host = optional_var(vars, "APP_HOST", "127.0.0.1");
        let port = parse_u16(vars, "APP_PORT", 8080)?;

        let qdrant_url = optional_var(vars, "QDRANT_URL", "http://localhost:6333");
        let knowledge_dir = optional_var(vars, "KNOWLEDGE_DIR", "./knowledge");

        let internal_api = InternalApiConfig {
            base_url: required_var(vars, "INTERNAL_API_BASE_URL")?,
            token: required_var(vars, "INTERNAL_API_TOKEN")?,
            chat_path: optional_var(vars, "INTERNAL_API_CHAT_PATH", "/v1/chat/completions"),
            embed_path: optional_var(vars, "INTERNAL_API_EMBED_PATH", "/v1/embeddings"),
            chat_model: optional_var(vars, "INTERNAL_API_CHAT_MODEL", "ad-qa-chat-v1"),
            embed_model: optional_var(vars, "INTERNAL_API_EMBED_MODEL", "ad-embed-v1"),
            llm_timeout_ms: parse_u64(vars, "LLM_TIMEOUT_MS", 2200)?,
            embed_timeout_ms: parse_u64(vars, "EMBED_TIMEOUT_MS", 5000)?,
            outbound_max_concurrency: parse_usize(vars, "OUTBOUND_MAX_CONCURRENCY", 8)?,
            chat_rate_limit_rpm: parse_u32(vars, "CHAT_RATE_LIMIT_RPM", 120)?,
            chat_burst: parse_u32(vars, "CHAT_BURST", 10)?,
            retry_chat_max: parse_u32(vars, "RETRY_CHAT_MAX", 1)?,
            retry_embed_max: parse_u32(vars, "RETRY_EMBED_MAX", 3)?,
        };

        Ok(Self {
            host,
            port,
            infer_provider: optional_var(vars, "INFER_PROVIDER", "internal_api"),
            qdrant_url,
            knowledge_dir,
            internal_api,
        })
    }
}

fn required_var(vars: &HashMap<String, String>, key: &'static str) -> Result<String, ConfigError> {
    vars.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::MissingEnv(key))
}

fn optional_var(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: &'static str,
) -> String {
    vars.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_u16(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u16,
) -> Result<u16, ConfigError> {
    match vars.get(key).map(|value| value.trim()) {
        Some(raw) if !raw.is_empty() => raw.parse::<u16>().map_err(|_| ConfigError::InvalidEnv {
            key,
            value: raw.to_string(),
            reason: "expected unsigned 16-bit integer",
        }),
        _ => Ok(default),
    }
}

fn parse_u32(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    match vars.get(key).map(|value| value.trim()) {
        Some(raw) if !raw.is_empty() => raw.parse::<u32>().map_err(|_| ConfigError::InvalidEnv {
            key,
            value: raw.to_string(),
            reason: "expected unsigned 32-bit integer",
        }),
        _ => Ok(default),
    }
}

fn parse_u64(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    match vars.get(key).map(|value| value.trim()) {
        Some(raw) if !raw.is_empty() => raw.parse::<u64>().map_err(|_| ConfigError::InvalidEnv {
            key,
            value: raw.to_string(),
            reason: "expected unsigned 64-bit integer",
        }),
        _ => Ok(default),
    }
}

fn parse_usize(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: usize,
) -> Result<usize, ConfigError> {
    match vars.get(key).map(|value| value.trim()) {
        Some(raw) if !raw.is_empty() => raw.parse::<usize>().map_err(|_| ConfigError::InvalidEnv {
            key,
            value: raw.to_string(),
            reason: "expected usize integer",
        }),
        _ => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{AppConfig, ConfigError};

    fn minimum_env() -> HashMap<String, String> {
        HashMap::from([
            (
                "INTERNAL_API_BASE_URL".to_string(),
                "https://internal-api.example.com".to_string(),
            ),
            ("INTERNAL_API_TOKEN".to_string(), "token-value".to_string()),
        ])
    }

    #[test]
    fn missing_internal_api_base_url_fails() {
        let mut vars = minimum_env();
        vars.remove("INTERNAL_API_BASE_URL");

        let result = AppConfig::from_map(&vars);
        assert_eq!(
            result.unwrap_err(),
            ConfigError::MissingEnv("INTERNAL_API_BASE_URL")
        );
    }

    #[test]
    fn missing_internal_api_token_fails() {
        let mut vars = minimum_env();
        vars.remove("INTERNAL_API_TOKEN");

        let result = AppConfig::from_map(&vars);
        assert_eq!(
            result.unwrap_err(),
            ConfigError::MissingEnv("INTERNAL_API_TOKEN")
        );
    }

    #[test]
    fn loads_defaults_when_optional_env_absent() {
        let vars = minimum_env();
        let config = AppConfig::from_map(&vars).expect("config should load");

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.infer_provider, "internal_api");
        assert_eq!(config.internal_api.chat_path, "/v1/chat/completions");
        assert_eq!(config.internal_api.embed_model, "ad-embed-v1");
        assert_eq!(config.internal_api.retry_embed_max, 3);
    }
}
