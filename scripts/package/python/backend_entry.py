"""PyInstaller entrypoint for EngineQA Python backend."""

from __future__ import annotations

import os

import uvicorn

from app.main import app


def _port_from_env(default: int = 8080) -> int:
    raw = os.getenv("APP_PORT", str(default)).strip()
    try:
        return int(raw)
    except ValueError:
        return default


if __name__ == "__main__":
    uvicorn.run(
        app,
        host=os.getenv("APP_HOST", "0.0.0.0"),
        port=_port_from_env(),
        log_level=os.getenv("LOG_LEVEL", "info").lower(),
    )

