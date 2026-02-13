from __future__ import annotations

from dataclasses import dataclass
import hashlib
from pathlib import Path
import time
import uuid
import logging

from qdrant_client.http import models

from .models import IndexResult
from .provider import InternalApiProvider
from .retriever import VectorRetriever


DEFAULT_CHUNK_SIZE = 1000
DEFAULT_OVERLAP = 125
UPSERT_BATCH_SIZE = 32
logger = logging.getLogger(__name__)


@dataclass
class _RawChunk:
    doc_id: str
    path: str
    title_path: str
    section: str
    text: str
    text_hash: str


class MarkdownIndexer:
    def __init__(
        self,
        provider: InternalApiProvider,
        retriever: VectorRetriever,
        knowledge_dir: str,
        chunk_size: int = DEFAULT_CHUNK_SIZE,
        overlap: int = DEFAULT_OVERLAP,
    ) -> None:
        self._provider = provider
        self._retriever = retriever
        self._knowledge_dir = Path(knowledge_dir)
        self._chunk_size = chunk_size
        self._overlap = overlap

    async def index(self) -> IndexResult:
        started_at = time.perf_counter()

        files = self._scan_markdown_files()
        deleted_chunks = self._reset_collection()

        total_files = len(files)
        indexed_files = 0
        failed_files = 0
        total_chunks = 0
        successful_chunks = 0
        failed_chunks = 0

        buffer: list[models.PointStruct] = []

        for path in files:
            try:
                content = path.read_text(encoding="utf-8")
            except OSError:
                failed_files += 1
                continue

            indexed_files += 1

            rel_path = self._relative_path(path)
            doc_id = _sha256(rel_path)
            raw_chunks = self._parse_and_chunk(content, rel_path, doc_id)

            for idx, chunk in enumerate(raw_chunks):
                total_chunks += 1
                try:
                    vector = await self._provider.embed(chunk.text)
                    point = models.PointStruct(
                        id=str(uuid.uuid5(uuid.NAMESPACE_URL, f"{chunk.doc_id}:{idx}:{chunk.text_hash}")),
                        vector=vector,
                        payload={
                            "doc_id": chunk.doc_id,
                            "path": chunk.path,
                            "title_path": chunk.title_path,
                            "section": chunk.section,
                            "text": chunk.text,
                            "hash": chunk.text_hash,
                        },
                    )
                    buffer.append(point)
                    successful_chunks += 1
                    if len(buffer) >= UPSERT_BATCH_SIZE:
                        self._flush_points(buffer)
                        buffer.clear()
                except Exception:
                    failed_chunks += 1

        if buffer:
            self._flush_points(buffer)

        duration_ms = int((time.perf_counter() - started_at) * 1000)

        if indexed_files > 0 and total_chunks == 0:
            logger.warning(
                "reindex_no_chunks total_files=%s indexed_files=%s reason=markdown_contains_only_headings_or_empty_sections",
                total_files,
                indexed_files,
            )

        return IndexResult(
            total_files=total_files,
            indexed_files=indexed_files,
            skipped_files=0,
            failed_files=failed_files,
            total_chunks=total_chunks,
            successful_chunks=successful_chunks,
            failed_chunks=failed_chunks,
            deleted_chunks=deleted_chunks,
            duration_ms=duration_ms,
        )

    def _scan_markdown_files(self) -> list[Path]:
        if not self._knowledge_dir.exists():
            return []
        return sorted(self._knowledge_dir.rglob("*.md"))

    def _reset_collection(self) -> int:
        deleted_chunks = 0
        try:
            deleted_chunks = self._retriever.count_points()
        except Exception:
            deleted_chunks = 0

        client = self._retriever.client
        collection = self._retriever.collection
        if client.collection_exists(collection_name=collection):
            client.delete_collection(collection_name=collection)
        self._retriever.ensure_collection_exists()
        return deleted_chunks

    def _flush_points(self, points: list[models.PointStruct]) -> None:
        self._retriever.client.upsert(
            collection_name=self._retriever.collection,
            points=points,
            wait=True,
        )

    def _relative_path(self, path: Path) -> str:
        try:
            return str(path.relative_to(self._knowledge_dir))
        except ValueError:
            return str(path)

    def _parse_and_chunk(self, content: str, rel_path: str, doc_id: str) -> list[_RawChunk]:
        lines = content.splitlines()
        heading_stack: list[str] = []
        current_section = ""
        current_title_path = ""
        buffer: list[str] = []
        raw_sections: list[_RawChunk] = []

        def flush_buffer() -> None:
            nonlocal buffer
            text = "\n".join(buffer).strip()
            buffer = []
            if not text:
                return

            for piece in self._split_with_overlap(text):
                raw_sections.append(
                    _RawChunk(
                        doc_id=doc_id,
                        path=rel_path,
                        title_path=current_title_path,
                        section=current_section,
                        text=piece,
                        text_hash=_sha256(piece),
                    )
                )

        for line in lines:
            stripped = line.strip()
            if stripped.startswith("#"):
                hashes = len(stripped) - len(stripped.lstrip("#"))
                title = stripped[hashes:].strip()
                if hashes > 0 and title:
                    flush_buffer()
                    if hashes <= 3:
                        heading_stack[:] = heading_stack[: hashes - 1]
                        heading_stack.append(title)
                        current_title_path = " / ".join(heading_stack)
                    current_section = title
                    # Index heading context as part of the section so
                    # heading-only markdown files still generate retrievable chunks.
                    buffer = [current_title_path or current_section]
                    continue
            buffer.append(line)

        flush_buffer()
        return raw_sections

    def _split_with_overlap(self, text: str) -> list[str]:
        chars = list(text)
        total = len(chars)
        if total <= self._chunk_size:
            return [text]

        result: list[str] = []
        step = max(1, self._chunk_size - self._overlap)
        start = 0
        while start < total:
            end = min(total, start + self._chunk_size)
            piece = "".join(chars[start:end]).strip()
            if piece:
                result.append(piece)
            if end >= total:
                break
            start += step
        return result


def _sha256(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()
