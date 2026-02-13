from __future__ import annotations

from enum import Enum
from pydantic import BaseModel, Field


class QueryRequest(BaseModel):
    question: str = Field(min_length=1)
    top_k: int = Field(default=6, ge=1, le=20)


class QuerySource(BaseModel):
    title: str
    path: str
    snippet: str
    score: float


class QueryResponse(BaseModel):
    answer: str
    sources: list[QuerySource]
    degraded: bool
    error_code: str | None = None
    trace_id: str


class FeedbackRating(str, Enum):
    useful = "useful"
    useless = "useless"


class FeedbackRequest(BaseModel):
    question: str = Field(min_length=1)
    answer: str = Field(min_length=1)
    rating: FeedbackRating
    comment: str | None = None
    error_code: str | None = None
    trace_id: str = Field(min_length=1)


class FeedbackResponse(BaseModel):
    ok: bool
    id: str


class RateLimitState(BaseModel):
    rpm_limit: int
    current_rpm: int


class StatusResponse(BaseModel):
    provider: str
    model: str
    index_size: int
    last_index_time: str | None = None
    upstream_health: str
    rate_limit_state: RateLimitState
    qdrant_connected: bool


class IndexResult(BaseModel):
    total_files: int
    indexed_files: int
    skipped_files: int
    failed_files: int
    total_chunks: int
    successful_chunks: int
    failed_chunks: int
    deleted_chunks: int
    duration_ms: int


class JobInfo(BaseModel):
    job_id: str
    status: str
    started_at: str
    ended_at: str | None = None
    result: IndexResult | None = None
    error: str | None = None


class ReindexRequest(BaseModel):
    pass


class ReindexResponse(BaseModel):
    job_id: str
    message: str


class ReindexStatusResponse(BaseModel):
    job: JobInfo | None = None
