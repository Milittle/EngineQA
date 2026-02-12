# EngineQA

广告引擎现网维优 QA 问答系统（Internal API 推理版）。

## Directory Layout
- `frontend/`: React + Vite + Tailwind app
- `backend/`: Axum API (Rust)
- `deploy/`: qdrant compose file
- `scripts/`: local startup and smoke validation scripts

## Ports
- Frontend: `5173`
- Backend: `8080`
- Qdrant HTTP: `6333`
- Qdrant gRPC: `6334`

## Quick Start (Host-Run, Recommended)
1. Install frontend deps:
```bash
npm install --prefix frontend
```

2. Copy env file:
```bash
cp .env.example .env
```

Then set required values in `.env`:
- `INTERNAL_API_BASE_URL` - Internal API base URL
- `INTERNAL_API_TOKEN` - Service token for API access
- `QDRANT_URL` - Qdrant server URL (default: http://localhost:6333)

3. Start qdrant on host (requires `qdrant` binary in PATH):
```bash
./scripts/run-qdrant.sh
```

Or via Docker Compose:
```bash
docker compose -f deploy/qdrant-compose.yaml up -d
```

4. Start frontend + backend in one command:
```bash
make dev
```

## API Endpoints

### `GET /health`
Health check endpoint.

### `POST /api/query`
Main QA query endpoint.

Request:
```json
{
  "question": "为什么广告请求QPS突然下降？",
  "top_k": 6
}
```

Response:
```json
{
  "answer": "可能原因包括...",
  "sources": [
    {
      "title": "竞价链路排障手册",
      "path": "docs/ops/bidding-debug.md",
      "snippet": "当QPS下降时，先看...",
      "score": 0.82
    }
  ],
  "degraded": false,
  "error_code": null,
  "trace_id": "req_20260211_xxx"
}
```

### `GET /api/status`
System status endpoint.

Response:
```json
{
  "provider": "internal_api",
  "model": "ad-qa-chat-v1",
  "index_size": 128734,
  "last_index_time": "2026-02-10T02:10:00Z",
  "upstream_health": "ok",
  "rate_limit_state": {
    "rpm_limit": 120,
    "current_rpm": 43
  },
  "qdrant_connected": true
}
```

### `POST /api/feedback`
Submit feedback for a query.

Request:
```json
{
  "question": "...",
  "answer": "...",
  "rating": "useful",
  "comment": "定位很快",
  "error_code": null,
  "trace_id": "req_20260211_xxx"
}
```

Response:
```json
{
  "ok": true,
  "id": "feedback_id"
}
```

### `POST /api/reindex`
Trigger reindexing job.

### `GET /api/reindex`
Get current reindex job status.

## Frontend Pages

### 问答页
- 输入问题并获取答案
- 显示参考来源和相关度
- 降级模式提示
- 反馈交互（有用/无用）
- 自动保存到历史记录

### 历史页
- 查看最近的问答历史
- 支持展开/收起答案
- 删除单个记录
- 清空所有记录
- 本地存储（localStorage）

### 状态页
- 推理服务健康状态
- 知识库索引规模
- 速率限制状态
- 触发重新索引
- 查看索引任务结果

## Environment Variables

See `.env.example` for all available configuration options.

## Smoke Check (5-10 minutes)
Run this after all services are up:

```bash
./scripts/smoke-step-01.sh
```

If qdrant is intentionally not started, you can skip qdrant check:

```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
```
