#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_PORT="${APP_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-5173}"

if [[ ! -d "${ROOT_DIR}/frontend/node_modules" ]]; then
  echo "frontend dependencies are missing. run: npm install --prefix frontend"
  exit 1
fi

cleanup() {
  if [[ -n "${backend_pid:-}" ]] && kill -0 "${backend_pid}" 2>/dev/null; then
    kill "${backend_pid}" 2>/dev/null || true
  fi
  if [[ -n "${frontend_pid:-}" ]] && kill -0 "${frontend_pid}" 2>/dev/null; then
    kill "${frontend_pid}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

(
  cd "${ROOT_DIR}"
  cargo run --manifest-path backend/Cargo.toml
) &
backend_pid=$!

(
  cd "${ROOT_DIR}"
  npm run dev --prefix frontend -- --host 0.0.0.0 --port "${FRONTEND_PORT}"
) &
frontend_pid=$!

echo "backend  -> http://127.0.0.1:${BACKEND_PORT}/health"
echo "frontend -> http://127.0.0.1:${FRONTEND_PORT}"
echo "press Ctrl+C to stop"

wait -n "${backend_pid}" "${frontend_pid}"
