from __future__ import annotations

import asyncio
from contextlib import asynccontextmanager
import logging
import os
import time
import uuid

from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware

from .config import AppConfig, ConfigError
from .errors import (
    ErrorCode,
    ProviderError,
    get_error_description,
    map_provider_error,
    should_degrade,
)
from .indexer import MarkdownIndexer
from .models import (
    FeedbackRequest,
    FeedbackResponse,
    QueryRequest,
    QueryResponse,
    QuerySource,
    RateLimitState,
    ReindexRequest,
    ReindexResponse,
    ReindexStatusResponse,
    StatusResponse,
)
from .provider import InternalApiProvider
from .retriever import RetrievedChunk, VectorRetriever
from .state import AppState, FeedbackStore, JobInProgressError, JobManager

logger = logging.getLogger(__name__)


CHAT_TEMPERATURE = 0.2
MAX_TOKENS = 65535
SYSTEM_PROMPT = """
你是一个广告引擎维优专家的智能助手。

1. 仅基于提供的参考资料回答。
2. 资料不足时明确说明“根据现有资料，我不确定”。
3. 不要编造内容。
4. 排障建议使用步骤化表达。
""".strip()


def _init_logging() -> None:
    root_logger = logging.getLogger()
    if root_logger.handlers:
        return

    level = os.getenv("LOG_LEVEL", "INFO").upper()
    logging.basicConfig(
        level=getattr(logging, level, logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s %(message)s",
    )


def _build_state() -> AppState:
    load_dotenv()
    _init_logging()
    config = AppConfig.from_env()

    logger.info(
        "config_loaded provider=%s qdrant_mode=%s qdrant_collection=%s embedding_vector_size=%s chat_base=%s chat_path=%s embed_base=%s embed_path=%s",
        config.infer_provider,
        config.qdrant_mode,
        config.qdrant_collection,
        config.embedding_vector_size,
        config.internal_api.chat_base_url,
        config.internal_api.chat_path,
        config.internal_api.embed_base_url,
        config.internal_api.embed_path,
    )

    provider = InternalApiProvider(config.internal_api)
    retriever = VectorRetriever(config)
    retriever.ensure_collection_exists()

    indexer = MarkdownIndexer(
        provider=provider,
        retriever=retriever,
        knowledge_dir=config.knowledge_dir,
    )

    return AppState(
        config=config,
        provider=provider,
        retriever=retriever,
        indexer=indexer,
        feedback_store=FeedbackStore(),
        job_manager=JobManager(),
    )


try:
    _STATE = _build_state()
except ConfigError as exc:
    raise RuntimeError(str(exc)) from exc


@asynccontextmanager
async def lifespan(app: FastAPI):
    app.state.engineqa = _STATE
    yield
    await _STATE.provider.close()


app = FastAPI(title="EngineQA Python Backend", lifespan=lifespan)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


def _state(request: Request) -> AppState:
    return request.app.state.engineqa


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/api/query", response_model=QueryResponse)
async def query(request: Request, req: QueryRequest) -> QueryResponse:
    state = _state(request)
    trace_id = str(uuid.uuid4())
    question = req.question.strip()
    start_time = time.perf_counter()

    logger.info(
        "query_received trace_id=%s top_k=%s question_length=%s",
        trace_id,
        req.top_k,
        len(question),
    )

    try:
        query_vector = await state.provider.embed(question, trace_id=trace_id)
    except ProviderError as exc:
        error_code = map_provider_error(exc)
        logger.warning(
            "query_embed_failed trace_id=%s error_code=%s provider_kind=%s provider_status=%s provider_error=%s",
            trace_id,
            error_code.value,
            exc.kind,
            exc.status_code,
            str(exc),
        )
        return _build_degraded_response(trace_id, error_code, [])

    try:
        chunks = state.retriever.retrieve(query_vector=query_vector, top_k=req.top_k)
    except Exception as exc:
        logger.exception(
            "query_retrieval_failed trace_id=%s error=%s",
            trace_id,
            str(exc),
        )
        return _build_degraded_response(trace_id, ErrorCode.RETRIEVAL_FAILED, [])

    if not chunks:
        logger.info("query_no_match trace_id=%s", trace_id)
        return _build_no_match_response(trace_id)

    context = _build_context(chunks)
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": f"问题: {question}\n\n参考资料:\n{context}"},
    ]

    sources = [_chunk_to_source(chunk) for chunk in chunks]

    try:
        answer = await state.provider.chat(
            messages=messages,
            temperature=CHAT_TEMPERATURE,
            max_tokens=MAX_TOKENS,
            trace_id=trace_id,
        )
    except ProviderError as exc:
        error_code = map_provider_error(exc)
        logger.warning(
            "query_chat_failed trace_id=%s error_code=%s provider_kind=%s provider_status=%s provider_error=%s",
            trace_id,
            error_code.value,
            exc.kind,
            exc.status_code,
            str(exc),
        )
        if should_degrade(error_code):
            return _build_degraded_with_sources_response(trace_id, error_code, sources)
        return _build_degraded_response(trace_id, error_code, [])

    elapsed_ms = int((time.perf_counter() - start_time) * 1000)
    logger.info(
        "query_completed trace_id=%s degraded=false sources_count=%s duration_ms=%s",
        trace_id,
        len(sources),
        elapsed_ms,
    )

    return QueryResponse(
        answer=answer,
        sources=sources,
        degraded=False,
        error_code=None,
        trace_id=trace_id,
    )


@app.get("/api/status", response_model=StatusResponse)
async def status(request: Request) -> StatusResponse:
    state = _state(request)

    qdrant_connected = True
    index_size = 0
    try:
        index_size = state.retriever.count_points()
    except Exception as exc:
        qdrant_connected = False
        logger.warning("status_qdrant_check_failed error=%s", str(exc))

    current_rpm = await state.provider.current_rpm()
    last_index_time = await state.job_manager.get_last_index_time()

    return StatusResponse(
        provider=state.config.infer_provider,
        model=state.config.internal_api.chat_model,
        index_size=index_size,
        last_index_time=last_index_time,
        upstream_health="ok",
        rate_limit_state=RateLimitState(
            rpm_limit=state.config.internal_api.chat_rate_limit_rpm,
            current_rpm=current_rpm,
        ),
        qdrant_connected=qdrant_connected,
    )


@app.post("/api/feedback", response_model=FeedbackResponse)
async def feedback(request: Request, req: FeedbackRequest) -> FeedbackResponse:
    state = _state(request)
    record_id = await state.feedback_store.save(req)
    logger.info("feedback_saved trace_id=%s rating=%s", req.trace_id, req.rating.value)
    return FeedbackResponse(ok=True, id=record_id)


@app.post("/api/reindex", response_model=ReindexResponse)
async def reindex(request: Request, _req: ReindexRequest) -> ReindexResponse:
    state = _state(request)

    try:
        job_id = await state.job_manager.start_job()
    except JobInProgressError as exc:
        logger.warning("reindex_rejected reason=job_in_progress")
        raise HTTPException(status_code=409, detail=str(exc)) from exc

    logger.info("reindex_started job_id=%s", job_id)
    asyncio.create_task(_run_reindex_job(state))

    return ReindexResponse(
        job_id=job_id,
        message="Reindex job started successfully",
    )


@app.get("/api/reindex", response_model=ReindexStatusResponse)
async def reindex_status(request: Request) -> ReindexStatusResponse:
    state = _state(request)
    job = await state.job_manager.get_current_job()
    return ReindexStatusResponse(job=job)


async def _run_reindex_job(state: AppState) -> None:
    try:
        result = await state.indexer.index()
        logger.info(
            "reindex_completed successful_chunks=%s failed_chunks=%s duration_ms=%s",
            result.successful_chunks,
            result.failed_chunks,
            result.duration_ms,
        )
        await state.job_manager.complete_job(result)
    except Exception as exc:
        logger.exception("reindex_failed error=%s", str(exc))
        await state.job_manager.fail_job(str(exc))


def _chunk_to_source(chunk: RetrievedChunk) -> QuerySource:
    return QuerySource(
        title=chunk.metadata.title_path,
        path=chunk.metadata.path,
        snippet=chunk.snippet,
        score=chunk.score,
    )


def _build_context(chunks: list[RetrievedChunk]) -> str:
    lines: list[str] = []
    for idx, chunk in enumerate(chunks, start=1):
        lines.append(
            "\n".join(
                [
                    f"[来源{idx}] {chunk.metadata.title_path}",
                    f"路径: {chunk.metadata.path}",
                    f"内容: {chunk.snippet}",
                ]
            )
        )
    return "\n\n".join(lines)


def _build_no_match_response(trace_id: str) -> QueryResponse:
    return QueryResponse(
        answer="根据现有知识库，我没有找到相关的参考资料来回答这个问题。",
        sources=[],
        degraded=True,
        error_code=ErrorCode.NO_MATCH.value,
        trace_id=trace_id,
    )


def _build_degraded_response(
    trace_id: str,
    error_code: ErrorCode,
    sources: list[QuerySource],
) -> QueryResponse:
    description = get_error_description(error_code)
    return QueryResponse(
        answer=f"服务暂时不可用：{description}。",
        sources=sources,
        degraded=True,
        error_code=error_code.value,
        trace_id=trace_id,
    )


def _build_degraded_with_sources_response(
    trace_id: str,
    error_code: ErrorCode,
    sources: list[QuerySource],
) -> QueryResponse:
    description = get_error_description(error_code)
    source_lines = "\n".join([f"- [{s.title}] {s.path}" for s in sources])
    answer = (
        f"AI 生成服务暂时不可用：{description}。\n\n"
        f"以下是相关参考文档，可先人工排查：\n{source_lines}"
    )
    return QueryResponse(
        answer=answer,
        sources=sources,
        degraded=True,
        error_code=error_code.value,
        trace_id=trace_id,
    )
