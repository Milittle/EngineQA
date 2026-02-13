from __future__ import annotations

from dataclasses import dataclass

from qdrant_client import QdrantClient
from qdrant_client.http import models

from .config import AppConfig


SCORE_THRESHOLD = 0.3


@dataclass
class ChunkMetadata:
    doc_id: str
    path: str
    title_path: str
    section: str


@dataclass
class RetrievedChunk:
    metadata: ChunkMetadata
    snippet: str
    score: float


class VectorRetriever:
    def __init__(self, config: AppConfig) -> None:
        self._collection = config.qdrant_collection
        self._vector_size = config.embedding_vector_size
        self._client = self._build_client(config)

    @property
    def client(self) -> QdrantClient:
        return self._client

    @property
    def collection(self) -> str:
        return self._collection

    def _build_client(self, config: AppConfig) -> QdrantClient:
        if config.qdrant_mode == "embedded":
            return QdrantClient(path=config.qdrant_local_path)
        return QdrantClient(url=config.qdrant_url)

    def ensure_collection_exists(self, vector_size: int | None = None) -> None:
        target_vector_size = vector_size or self._vector_size

        if self._client.collection_exists(collection_name=self._collection):
            current_size = self._collection_vector_size()
            if current_size == target_vector_size:
                return
            self._client.delete_collection(collection_name=self._collection)

        self._client.create_collection(
            collection_name=self._collection,
            vectors_config=models.VectorParams(
                size=target_vector_size,
                distance=models.Distance.COSINE,
            ),
            optimizers_config=models.OptimizersConfigDiff(indexing_threshold=20000),
        )

    def retrieve(self, query_vector: list[float], top_k: int = 6) -> list[RetrievedChunk]:
        query_response = self._client.query_points(
            collection_name=self._collection,
            query=query_vector,
            limit=top_k,
            with_payload=True,
        )
        results = query_response.points

        chunks: list[RetrievedChunk] = []
        for hit in results:
            score = float(hit.score or 0)
            if score < SCORE_THRESHOLD:
                continue

            payload = hit.payload or {}
            doc_id = _payload_str(payload, "doc_id")
            path = _payload_str(payload, "path")
            title_path = _payload_str(payload, "title_path")
            section = _payload_str(payload, "section")
            snippet = _payload_str(payload, "text")

            chunks.append(
                RetrievedChunk(
                    metadata=ChunkMetadata(
                        doc_id=doc_id,
                        path=path,
                        title_path=title_path,
                        section=section,
                    ),
                    snippet=snippet,
                    score=score,
                )
            )

        return chunks

    def count_points(self) -> int:
        if not self._client.collection_exists(collection_name=self._collection):
            return 0
        count = self._client.count(
            collection_name=self._collection,
            count_filter=None,
            exact=False,
        )
        return int(count.count)

    def _collection_vector_size(self) -> int | None:
        info = self._client.get_collection(collection_name=self._collection)
        vectors = info.config.params.vectors
        if vectors is None:
            return None
        if hasattr(vectors, "size"):
            return int(vectors.size)
        return None


def _payload_str(payload: dict, key: str) -> str:
    value = payload.get(key)
    if value is None:
        return ""
    return str(value)
