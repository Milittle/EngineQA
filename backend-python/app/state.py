from __future__ import annotations

import asyncio
from datetime import datetime, timezone
import uuid

from .config import AppConfig
from .indexer import MarkdownIndexer
from .models import FeedbackRequest, IndexResult, JobInfo
from .provider import InternalApiProvider
from .retriever import VectorRetriever


class FeedbackStore:
    def __init__(self) -> None:
        self._records: list[dict] = []
        self._lock = asyncio.Lock()

    async def save(self, request: FeedbackRequest) -> str:
        record_id = str(uuid.uuid4())
        record = {
            "id": record_id,
            "question": request.question,
            "answer": request.answer,
            "rating": request.rating.value,
            "comment": request.comment,
            "error_code": request.error_code,
            "trace_id": request.trace_id,
            "created_at": _utc_now(),
        }
        async with self._lock:
            self._records.append(record)
        return record_id


class JobInProgressError(RuntimeError):
    pass


class JobManager:
    def __init__(self) -> None:
        self._lock = asyncio.Lock()
        self._current_job: JobInfo | None = None
        self._last_index_time: str | None = None

    async def start_job(self) -> str:
        async with self._lock:
            if self._current_job and self._current_job.status == "running":
                raise JobInProgressError("Another reindex job is already in progress")

            job_id = str(uuid.uuid4())
            self._current_job = JobInfo(
                job_id=job_id,
                status="running",
                started_at=_utc_now(),
                ended_at=None,
                result=None,
                error=None,
            )
            return job_id

    async def complete_job(self, result: IndexResult) -> None:
        async with self._lock:
            if not self._current_job:
                return
            ended_at = _utc_now()
            self._current_job.status = "completed"
            self._current_job.ended_at = ended_at
            self._current_job.result = result
            self._current_job.error = None
            self._last_index_time = ended_at

    async def fail_job(self, error_message: str) -> None:
        async with self._lock:
            if not self._current_job:
                return
            self._current_job.status = "failed"
            self._current_job.ended_at = _utc_now()
            self._current_job.error = error_message

    async def get_current_job(self) -> JobInfo | None:
        async with self._lock:
            if self._current_job is None:
                return None
            return JobInfo.model_validate(self._current_job.model_dump())

    async def get_last_index_time(self) -> str | None:
        async with self._lock:
            return self._last_index_time


class AppState:
    def __init__(
        self,
        config: AppConfig,
        provider: InternalApiProvider,
        retriever: VectorRetriever,
        indexer: MarkdownIndexer,
        feedback_store: FeedbackStore,
        job_manager: JobManager,
    ) -> None:
        self.config = config
        self.provider = provider
        self.retriever = retriever
        self.indexer = indexer
        self.feedback_store = feedback_store
        self.job_manager = job_manager


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()
