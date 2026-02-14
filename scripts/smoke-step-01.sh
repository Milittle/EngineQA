#!/usr/bin/env bash
set -euo pipefail

BACKEND_HEALTH_URL="${BACKEND_HEALTH_URL:-http://127.0.0.1:8080/health}"
FRONTEND_URL="${FRONTEND_URL:-http://127.0.0.1:5173}"
STATUS_URL="${STATUS_URL:-http://127.0.0.1:8080/api/status}"
QDRANT_HEALTH_URL="${QDRANT_HEALTH_URL:-http://127.0.0.1:6333/healthz}"
BACKEND_RUNTIME="${BACKEND_RUNTIME:-python}"

# Backward compatibility: SKIP_QDRANT still works.
SKIP_VECTOR_CHECK="${SKIP_VECTOR_CHECK:-${SKIP_QDRANT:-0}}"

echo "checking backend: ${BACKEND_HEALTH_URL}"
curl -fsS "${BACKEND_HEALTH_URL}" >/tmp/engineqa-backend-health.json

echo "checking frontend: ${FRONTEND_URL}"
curl -fsS "${FRONTEND_URL}" >/tmp/engineqa-frontend-index.html

if [[ "${SKIP_VECTOR_CHECK}" != "1" ]]; then
  if [[ "${BACKEND_RUNTIME}" == "rust" ]]; then
    echo "checking vector store via status endpoint: ${STATUS_URL}"
    status_json="$(curl -fsS "${STATUS_URL}")"
    if echo "${status_json}" | grep -Eq '"vector_store_connected"[[:space:]]*:[[:space:]]*true'; then
      echo "vector store check passed (lancedb)"
    else
      echo "vector store check failed: vector_store_connected is not true"
      exit 1
    fi
  else
    echo "checking qdrant: ${QDRANT_HEALTH_URL}"
    curl -fsS "${QDRANT_HEALTH_URL}" >/tmp/engineqa-qdrant-health.json
  fi
fi

echo "step-01 smoke check passed"
