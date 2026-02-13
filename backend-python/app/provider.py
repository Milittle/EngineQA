from __future__ import annotations

import asyncio
from collections import deque
import logging
from typing import Any
import uuid

import httpx

from .config import InternalApiConfig
from .errors import ProviderError

logger = logging.getLogger(__name__)


class InternalApiProvider:
    def __init__(self, config: InternalApiConfig) -> None:
        self._config = config
        self._client = httpx.AsyncClient()
        self._request_times: deque[float] = deque()
        self._rpm_lock = asyncio.Lock()

    async def close(self) -> None:
        await self._client.aclose()

    async def current_rpm(self) -> int:
        async with self._rpm_lock:
            self._trim_request_window()
            return len(self._request_times)

    async def embed(self, text: str, trace_id: str | None = None) -> list[float]:
        payload = {
            "model": self._config.embed_model,
            "input": text,
        }
        response = await self._post_json(
            operation="embed",
            base_url=self._config.embed_base_url,
            token=self._config.embed_token,
            path=self._config.embed_path,
            payload=payload,
            timeout_ms=self._config.embed_timeout_ms,
            retry_max=self._config.retry_embed_max,
            trace_id=trace_id,
        )

        data = response.get("data")
        if not isinstance(data, list) or not data:
            raise ProviderError("parse", "embedding response missing data")

        first = data[0]
        embedding = first.get("embedding") if isinstance(first, dict) else None
        if not isinstance(embedding, list) or not embedding:
            raise ProviderError("parse", "embedding response missing vector")

        try:
            return [float(v) for v in embedding]
        except (TypeError, ValueError) as exc:
            raise ProviderError("parse", "embedding vector contains non-numeric values") from exc

    async def chat(
        self,
        messages: list[dict[str, str]],
        temperature: float,
        max_tokens: int,
        trace_id: str | None = None,
    ) -> str:
        payload = {
            "model": self._config.chat_model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }

        response = await self._post_json(
            operation="chat",
            base_url=self._config.chat_base_url,
            token=self._config.chat_token,
            path=self._config.chat_path,
            payload=payload,
            timeout_ms=self._config.llm_timeout_ms,
            retry_max=self._config.retry_chat_max,
            trace_id=trace_id,
        )

        choices = response.get("choices")
        if not isinstance(choices, list) or not choices:
            raise ProviderError("parse", "chat response missing choices")

        first = choices[0]
        message = first.get("message") if isinstance(first, dict) else None
        content = message.get("content") if isinstance(message, dict) else None
        if not isinstance(content, str) or not content.strip():
            raise ProviderError("parse", "chat response missing message content")

        return content.strip()

    async def _post_json(
        self,
        operation: str,
        base_url: str,
        token: str,
        path: str,
        payload: dict[str, Any],
        timeout_ms: int,
        retry_max: int,
        trace_id: str | None,
    ) -> dict[str, Any]:
        url = f"{base_url.rstrip('/')}{path}"
        request_id = trace_id or str(uuid.uuid4())

        for attempt in range(retry_max + 1):
            try:
                await self._record_request()
                response = await self._client.post(
                    url,
                    json=payload,
                    headers={
                        "Authorization": f"Bearer {token}",
                        "X-Request-Id": request_id,
                        "Content-Type": "application/json",
                    },
                    timeout=timeout_ms / 1000,
                )

                if response.status_code >= 400:
                    text = response.text[:500]
                    logger.warning(
                        "upstream_%s_failed trace_id=%s attempt=%s status=%s url=%s body=%s",
                        operation,
                        request_id,
                        attempt + 1,
                        response.status_code,
                        url,
                        text.replace("\n", "\\n"),
                    )
                    provider_error = ProviderError(
                        "api",
                        f"upstream api error ({response.status_code}): {text}",
                        status_code=response.status_code,
                    )
                    if attempt < retry_max and self._is_retryable(provider_error):
                        logger.warning(
                            "upstream_%s_retry trace_id=%s attempt=%s/%s",
                            operation,
                            request_id,
                            attempt + 1,
                            retry_max + 1,
                        )
                        await asyncio.sleep(0.5 * (attempt + 1))
                        continue
                    raise provider_error

                data = response.json()
                if not isinstance(data, dict):
                    raise ProviderError("parse", "upstream response is not a JSON object")
                return data
            except httpx.TimeoutException as exc:
                provider_error = ProviderError("timeout", "upstream request timeout")
                logger.warning(
                    "upstream_%s_timeout trace_id=%s attempt=%s/%s url=%s",
                    operation,
                    request_id,
                    attempt + 1,
                    retry_max + 1,
                    url,
                )
                if attempt < retry_max:
                    await asyncio.sleep(0.5 * (attempt + 1))
                    continue
                raise provider_error from exc
            except httpx.RequestError as exc:
                provider_error = ProviderError("request", f"upstream request failed: {exc}")
                logger.warning(
                    "upstream_%s_request_error trace_id=%s attempt=%s/%s url=%s error=%s",
                    operation,
                    request_id,
                    attempt + 1,
                    retry_max + 1,
                    url,
                    str(exc),
                )
                if attempt < retry_max:
                    await asyncio.sleep(0.5 * (attempt + 1))
                    continue
                raise provider_error from exc
            except ValueError as exc:
                logger.warning(
                    "upstream_%s_parse_error trace_id=%s url=%s error=%s",
                    operation,
                    request_id,
                    url,
                    str(exc),
                )
                raise ProviderError("parse", f"invalid upstream JSON: {exc}") from exc

        raise ProviderError("request", "upstream request exhausted retries")

    def _is_retryable(self, error: ProviderError) -> bool:
        if error.kind == "timeout":
            return True
        if error.kind == "request":
            return True
        if error.kind == "api" and error.status_code in {429, 500, 502, 503, 504}:
            return True
        return False

    async def _record_request(self) -> None:
        async with self._rpm_lock:
            self._trim_request_window()
            self._request_times.append(asyncio.get_running_loop().time())

    def _trim_request_window(self) -> None:
        now = asyncio.get_running_loop().time()
        cutoff = now - 60.0
        while self._request_times and self._request_times[0] < cutoff:
            self._request_times.popleft()
