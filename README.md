# EngineQA

Step-01 baseline scaffold for EngineQA with host-first local runtime.

## Directory Layout
- `frontend/`: React + Vite + Tailwind app scaffold
- `backend/`: Axum API scaffold (`GET /health`)
- `deploy/`: optional qdrant compose file
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
- `INTERNAL_API_BASE_URL`
- `INTERNAL_API_TOKEN`

3. Start qdrant on host (requires `qdrant` binary in PATH):
```bash
./scripts/run-qdrant.sh
```
4. Start frontend + backend in one command:
```bash
make dev
```

## Optional qdrant via Docker Compose
Only use this when host-run qdrant is unavailable.

```bash
docker compose -f deploy/qdrant-compose.yaml up -d
```

## Smoke Check (5-10 minutes)
Run this after all services are up:

```bash
./scripts/smoke-step-01.sh
```

If qdrant is intentionally not started, you can skip qdrant check:

```bash
SKIP_QDRANT=1 ./scripts/smoke-step-01.sh
```
