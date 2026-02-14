#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[notice] run-qdrant.sh is for Python backend remote mode only."
echo "[notice] Rust backend now uses LanceDB local storage and does not require qdrant process."

if ! command -v qdrant >/dev/null 2>&1; then
  echo "qdrant binary not found in PATH."
  echo "install qdrant for host-run, or use: docker compose -f deploy/qdrant-compose.yaml up -d"
  exit 1
fi

QDRANT_STORAGE_DIR="${QDRANT_STORAGE_DIR:-${ROOT_DIR}/.qdrant-storage}"
mkdir -p "${QDRANT_STORAGE_DIR}"

export QDRANT__STORAGE__STORAGE_PATH="${QDRANT_STORAGE_DIR}"
export QDRANT__SERVICE__HTTP_PORT="${QDRANT_HTTP_PORT:-6333}"
export QDRANT__SERVICE__GRPC_PORT="${QDRANT_GRPC_PORT:-6334}"

echo "starting qdrant at http://127.0.0.1:${QDRANT__SERVICE__HTTP_PORT}"
exec qdrant
