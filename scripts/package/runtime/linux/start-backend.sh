#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNTIME_ENV_FILE="${ROOT_DIR}/runtime.env"

if [[ -f "${RUNTIME_ENV_FILE}" ]]; then
  # shellcheck disable=SC1090
  source "${RUNTIME_ENV_FILE}"
fi

ENV_FILE="${APP_ENV_FILE:-${ROOT_DIR}/.env}"
if [[ ! -f "${ENV_FILE}" ]]; then
  echo "missing environment file: ${ENV_FILE}"
  echo "initialize with: cp config/.env.example .env"
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +a

BACKEND_BIN_REL="${ENGINEQA_BACKEND_BIN:-bin/engineqa-backend}"
BACKEND_BIN="${ROOT_DIR}/${BACKEND_BIN_REL}"
PID_FILE="${ROOT_DIR}/${ENGINEQA_PID_FILE:-run/backend.pid}"
LOG_FILE="${ROOT_DIR}/${ENGINEQA_LOG_FILE:-logs/backend.log}"
RUNTIME_KIND="${ENGINEQA_RUNTIME_KIND:-rust}"
FRONTEND_DIR="${ROOT_DIR}/${ENGINEQA_FRONTEND_DIR:-frontend}"
NGINX_PID_FILE="${ROOT_DIR}/${ENGINEQA_NGINX_PID_FILE:-run/nginx.pid}"
NGINX_CONF_FILE="${ROOT_DIR}/${ENGINEQA_NGINX_CONF_FILE:-run/nginx.conf}"
NGINX_ACCESS_LOG="${ROOT_DIR}/${ENGINEQA_NGINX_ACCESS_LOG:-logs/nginx.access.log}"
NGINX_ERROR_LOG="${ROOT_DIR}/${ENGINEQA_NGINX_ERROR_LOG:-logs/nginx.error.log}"
FRONTEND_PORT="${FRONTEND_PORT:-5173}"
BACKEND_PORT="${APP_PORT:-8080}"
NGINX_USER="${ENGINEQA_NGINX_USER:-$(id -un)}"

if [[ ! -x "${BACKEND_BIN}" ]]; then
  echo "backend executable not found: ${BACKEND_BIN}"
  exit 1
fi

if [[ ! -d "${FRONTEND_DIR}" ]]; then
  echo "frontend directory not found: ${FRONTEND_DIR}"
  exit 1
fi

mkdir -p \
  "$(dirname "${PID_FILE}")" \
  "$(dirname "${LOG_FILE}")" \
  "$(dirname "${NGINX_PID_FILE}")" \
  "$(dirname "${NGINX_CONF_FILE}")" \
  "$(dirname "${NGINX_ACCESS_LOG}")" \
  "$(dirname "${NGINX_ERROR_LOG}")" \
  "${ROOT_DIR}/data"

if [[ -f "${PID_FILE}" ]]; then
  existing_pid="$(cat "${PID_FILE}")"
  if [[ -n "${existing_pid}" ]] && kill -0 "${existing_pid}" 2>/dev/null; then
    echo "backend is already running (pid=${existing_pid})"
    exit 1
  fi
  rm -f "${PID_FILE}"
fi

if [[ -f "${NGINX_PID_FILE}" ]]; then
  existing_nginx_pid="$(cat "${NGINX_PID_FILE}")"
  if [[ -n "${existing_nginx_pid}" ]] && kill -0 "${existing_nginx_pid}" 2>/dev/null; then
    echo "frontend nginx is already running (pid=${existing_nginx_pid})"
    exit 1
  fi
  rm -f "${NGINX_PID_FILE}"
fi

export APP_HOST="${APP_HOST:-0.0.0.0}"

case "${RUNTIME_KIND}" in
  rust)
    export LANCEDB_URI="${LANCEDB_URI:-${ROOT_DIR}/data/.lancedb}"
    ;;
  python)
    export QDRANT_LOCAL_PATH="${QDRANT_LOCAL_PATH:-${ROOT_DIR}/data/.qdrant-local}"
    ;;
  *)
    echo "unknown runtime kind: ${RUNTIME_KIND}"
    exit 1
    ;;
esac

if ! command -v nginx >/dev/null 2>&1; then
  echo "nginx command not found in PATH"
  exit 1
fi

if ! id -u "${NGINX_USER}" >/dev/null 2>&1; then
  echo "invalid nginx runtime user: ${NGINX_USER}"
  exit 1
fi

nohup "${BACKEND_BIN}" >>"${LOG_FILE}" 2>&1 &
backend_pid="$!"
echo "${backend_pid}" > "${PID_FILE}"

cat > "${NGINX_CONF_FILE}" <<EOF
user ${NGINX_USER};
pid "${NGINX_PID_FILE}";

events {
  worker_connections 1024;
}

http {
  types {
    text/html html htm shtml;
    text/css css;
    text/xml xml;
    application/javascript js mjs;
    application/json json;
    image/svg+xml svg svgz;
    image/png png;
    image/jpeg jpeg jpg;
    image/x-icon ico;
    font/woff woff;
    font/woff2 woff2;
  }
  default_type application/octet-stream;

  access_log "${NGINX_ACCESS_LOG}";
  error_log "${NGINX_ERROR_LOG}" warn;

  server {
    listen ${FRONTEND_PORT};
    server_name 127.0.0.1 localhost;

    root "${FRONTEND_DIR}";
    index index.html;

    location /api/ {
      proxy_pass http://127.0.0.1:${BACKEND_PORT};
      proxy_http_version 1.1;
      proxy_set_header Host \$host;
      proxy_set_header X-Real-IP \$remote_addr;
      proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
      proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location = /health {
      proxy_pass http://127.0.0.1:${BACKEND_PORT}/health;
      proxy_http_version 1.1;
      proxy_set_header Host \$host;
    }

    location /assets/ {
      if_modified_since off;
      etag off;
      add_header Cache-Control "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0" always;
      try_files \$uri =404;
    }

    location / {
      try_files \$uri \$uri/ /index.html;
    }
  }
}
EOF

if ! nginx -g "error_log ${NGINX_ERROR_LOG};" -p "${ROOT_DIR}" -c "${NGINX_CONF_FILE}" >/dev/null 2>&1; then
  if kill -0 "${backend_pid}" 2>/dev/null; then
    kill "${backend_pid}" 2>/dev/null || true
  fi
  rm -f "${PID_FILE}"
  echo "failed to start nginx with config: ${NGINX_CONF_FILE}"
  exit 1
fi

echo "services started"
echo "pid: ${backend_pid}"
echo "backend health: http://127.0.0.1:${BACKEND_PORT}/health"
echo "frontend url: http://127.0.0.1:${FRONTEND_PORT}"
echo "backend logs: ${LOG_FILE}"
echo "nginx logs: ${NGINX_ACCESS_LOG}, ${NGINX_ERROR_LOG}"
