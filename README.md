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
