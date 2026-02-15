# EngineQA Python Backend

FastAPI implementation of EngineQA backend.

## 当前状态
- 这是当前默认可运行后端（2026-02-13）。
- Rust 后端暂不作为运行基线。
- Qdrant 仅支持 embedded 模式。

## 依赖安装
推荐方式（venv）：
```bash
python3 -m venv .venv-backend-python
.venv-backend-python/bin/pip install -r backend-python/requirements.txt
```

## 运行方式
### 方式 1：通过项目统一脚本（推荐）
```bash
BACKEND_RUNTIME=python make dev
```

### 方式 2：仅启动后端
```bash
.venv-backend-python/bin/python -m uvicorn app.main:app \
  --app-dir backend-python --host 0.0.0.0 --port 8080
```

## 必要环境变量
至少需要：
- `INTERNAL_API_BASE_URL`（或拆分的 chat/embed base url）
- `INTERNAL_API_TOKEN`

可选拆分配置：
- `INTERNAL_API_CHAT_BASE_URL`
- `INTERNAL_API_EMBED_BASE_URL`
- `INTERNAL_API_CHAT_PATH`
- `INTERNAL_API_EMBED_PATH`
- `INTERNAL_API_CHAT_TOKEN`
- `INTERNAL_API_EMBED_TOKEN`

## Qdrant 配置
- Embedded（默认且唯一支持）：
  - `QDRANT_LOCAL_PATH=./.qdrant-local`
  - `QDRANT_COLLECTION=knowledge_chunks`

## API
- `GET /health`
- `POST /api/query`
- `GET /api/status`
- `POST /api/feedback`
- `POST /api/reindex`
- `GET /api/reindex`

## 故障排查日志关键字
- `query_embed_failed`
- `query_chat_failed`
- `upstream_embed_failed`
- `upstream_chat_failed`
- `reindex_failed`
