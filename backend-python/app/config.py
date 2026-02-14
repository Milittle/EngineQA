from __future__ import annotations

from dataclasses import dataclass
import os


class ConfigError(RuntimeError):
    pass


def _required_env(key: str) -> str:
    value = os.getenv(key, "").strip()
    if not value:
        raise ConfigError(f"missing required env: {key}")
    return value


def _optional_env(key: str, default: str) -> str:
    value = os.getenv(key)
    if value is None:
        return default
    value = value.strip()
    return value or default


def _int_env(key: str, default: int) -> int:
    raw = os.getenv(key)
    if raw is None or not raw.strip():
        return default
    try:
        return int(raw)
    except ValueError as exc:
        raise ConfigError(f"invalid env {key}={raw}: expected integer") from exc


@dataclass(frozen=True)
class InternalApiConfig:
    base_url: str
    chat_base_url: str
    embed_base_url: str
    token: str
    chat_token: str
    embed_token: str
    chat_path: str
    embed_path: str
    chat_model: str
    embed_model: str
    embedding_vector_size: int
    llm_timeout_ms: int
    embed_timeout_ms: int
    outbound_max_concurrency: int
    chat_rate_limit_rpm: int
    chat_burst: int
    retry_chat_max: int
    retry_embed_max: int


@dataclass(frozen=True)
class AppConfig:
    host: str
    port: int
    infer_provider: str
    knowledge_dir: str
    qdrant_url: str
    qdrant_collection: str
    qdrant_mode: str
    qdrant_local_path: str
    embedding_vector_size: int
    internal_api: InternalApiConfig

    @classmethod
    def from_env(cls) -> "AppConfig":
        qdrant_mode = _optional_env("QDRANT_MODE", "embedded").lower()
        if qdrant_mode not in {"embedded", "remote"}:
            raise ConfigError(
                "invalid env QDRANT_MODE: expected one of embedded/remote"
            )

        base_url = _optional_env("INTERNAL_API_BASE_URL", "")
        chat_base_url = _optional_env("INTERNAL_API_CHAT_BASE_URL", base_url)
        embed_base_url = _optional_env("INTERNAL_API_EMBED_BASE_URL", base_url)
        if not chat_base_url:
            raise ConfigError(
                "missing required env: INTERNAL_API_CHAT_BASE_URL (or INTERNAL_API_BASE_URL)"
            )
        if not embed_base_url:
            raise ConfigError(
                "missing required env: INTERNAL_API_EMBED_BASE_URL (or INTERNAL_API_BASE_URL)"
            )

        shared_token = _required_env("INTERNAL_API_TOKEN")
        chat_token = _optional_env("INTERNAL_API_CHAT_TOKEN", shared_token)
        embed_token = _optional_env("INTERNAL_API_EMBED_TOKEN", shared_token)

        internal_api = InternalApiConfig(
            base_url=base_url,
            chat_base_url=chat_base_url,
            embed_base_url=embed_base_url,
            token=shared_token,
            chat_token=chat_token,
            embed_token=embed_token,
            chat_path=_optional_env("INTERNAL_API_CHAT_PATH", "/chat/completions"),
            embed_path=_optional_env("INTERNAL_API_EMBED_PATH", "/embeddings"),
            chat_model=_optional_env("INTERNAL_API_CHAT_MODEL", "GLM-4.7"),
            embed_model=_optional_env("INTERNAL_API_EMBED_MODEL", "embedding-3"),
            llm_timeout_ms=_int_env("LLM_TIMEOUT_MS", 2200),
            embed_timeout_ms=_int_env("EMBED_TIMEOUT_MS", 5000),
            outbound_max_concurrency=_int_env("OUTBOUND_MAX_CONCURRENCY", 8),
            chat_rate_limit_rpm=_int_env("CHAT_RATE_LIMIT_RPM", 120),
            chat_burst=_int_env("CHAT_BURST", 10),
            retry_chat_max=_int_env("RETRY_CHAT_MAX", 1),
            retry_embed_max=_int_env("RETRY_EMBED_MAX", 3),
            embedding_vector_size=_int_env("EMBEDDING_VECTOR_SIZE", 1536),
        )

        return cls(
            host=_optional_env("APP_HOST", "127.0.0.1"),
            port=_int_env("APP_PORT", 8080),
            infer_provider=_optional_env("INFER_PROVIDER", "internal_api"),
            knowledge_dir=_optional_env("KNOWLEDGE_DIR", "./knowledge"),
            qdrant_url=_optional_env("QDRANT_URL", "http://127.0.0.1:6333"),
            qdrant_collection=_optional_env("QDRANT_COLLECTION", "knowledge_chunks"),
            qdrant_mode=qdrant_mode,
            qdrant_local_path=_optional_env("QDRANT_LOCAL_PATH", "./.qdrant-local"),
            embedding_vector_size=_int_env("EMBEDDING_VECTOR_SIZE", 1536),
            internal_api=internal_api,
        )
