from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class ErrorCode(str, Enum):
    UPSTREAM_TIMEOUT = "UPSTREAM_TIMEOUT"
    UPSTREAM_RATE_LIMIT = "UPSTREAM_RATE_LIMIT"
    UPSTREAM_AUTH = "UPSTREAM_AUTH"
    UPSTREAM_UNAVAILABLE = "UPSTREAM_UNAVAILABLE"
    UPSTREAM_ERROR = "UPSTREAM_ERROR"
    RETRIEVAL_FAILED = "RETRIEVAL_FAILED"
    NO_MATCH = "NO_MATCH"
    INTERNAL_ERROR = "INTERNAL_ERROR"


@dataclass
class ProviderError(Exception):
    kind: str
    message: str
    status_code: int | None = None

    def __str__(self) -> str:
        return self.message


def map_status_code(status_code: int) -> ErrorCode:
    if status_code in (401, 403):
        return ErrorCode.UPSTREAM_AUTH
    if status_code == 429:
        return ErrorCode.UPSTREAM_RATE_LIMIT
    if status_code in (408, 504):
        return ErrorCode.UPSTREAM_TIMEOUT
    if status_code in (500, 502, 503):
        return ErrorCode.UPSTREAM_UNAVAILABLE
    if 400 <= status_code < 500:
        return ErrorCode.UPSTREAM_ERROR
    if 500 <= status_code < 600:
        return ErrorCode.UPSTREAM_UNAVAILABLE
    return ErrorCode.UPSTREAM_ERROR


def map_provider_error(error: ProviderError) -> ErrorCode:
    if error.kind == "timeout":
        return ErrorCode.UPSTREAM_TIMEOUT
    if error.kind == "api" and error.status_code is not None:
        return map_status_code(error.status_code)
    if error.kind == "request":
        return ErrorCode.UPSTREAM_UNAVAILABLE
    if error.kind == "parse":
        return ErrorCode.INTERNAL_ERROR
    return ErrorCode.UPSTREAM_ERROR


def get_error_description(code: ErrorCode) -> str:
    descriptions = {
        ErrorCode.UPSTREAM_TIMEOUT: "上游服务响应超时，请稍后重试",
        ErrorCode.UPSTREAM_RATE_LIMIT: "上游服务限流，请求过于频繁",
        ErrorCode.UPSTREAM_AUTH: "上游服务认证失败，请检查 API Token",
        ErrorCode.UPSTREAM_UNAVAILABLE: "上游服务不可用，请稍后重试",
        ErrorCode.UPSTREAM_ERROR: "上游服务返回错误",
        ErrorCode.RETRIEVAL_FAILED: "检索服务失败，请检查 Qdrant 配置",
        ErrorCode.NO_MATCH: "未找到相关资料，请尝试其他问题",
        ErrorCode.INTERNAL_ERROR: "内部服务错误，请联系技术团队",
    }
    return descriptions[code]


def should_degrade(code: ErrorCode) -> bool:
    return code in {
        ErrorCode.UPSTREAM_TIMEOUT,
        ErrorCode.UPSTREAM_RATE_LIMIT,
        ErrorCode.UPSTREAM_AUTH,
        ErrorCode.UPSTREAM_UNAVAILABLE,
        ErrorCode.UPSTREAM_ERROR,
    }
