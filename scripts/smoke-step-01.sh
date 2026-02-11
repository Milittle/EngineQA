#!/usr/bin/env bash
set -euo pipefail

BACKEND_HEALTH_URL="${BACKEND_HEALTH_URL:-http://127.0.0.1:8080/health}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:5173}"
QDRANT_HEALTH_URL="${QDRANT_HEALTH_URL:-http://127.0.0.1:6333/healthz}"

echo "checking backend: ${BACKEND_HEALTH_URL}"
curl -fsS "${BACKEND_HEALTH_URL}" >/tmp/engineqa-backend-health.json

echo "checking frontend: ${FRONTEND_URL}"
curl -fsS "${FRONTEND_URL}" >/tmp/engineqa-frontend-index.html

if [[ "${SKIP_QDRANT:-0}" != "1" ]]; then
  echo "checking qdrant: ${QDRANT_HEALTH_URL}"
  curl -fsS "${QDRANT_HEALTH_URL}" >/tmp/engineqa-qdrant-health.json
fi

echo "step-01 smoke check passed"
