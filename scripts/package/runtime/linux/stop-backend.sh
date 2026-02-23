#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNTIME_ENV_FILE="${ROOT_DIR}/runtime.env"

if [[ -f "${RUNTIME_ENV_FILE}" ]]; then
  # shellcheck disable=SC1090
  source "${RUNTIME_ENV_FILE}"
fi

PID_FILE="${ROOT_DIR}/${ENGINEQA_PID_FILE:-run/backend.pid}"
NGINX_PID_FILE="${ROOT_DIR}/${ENGINEQA_NGINX_PID_FILE:-run/nginx.pid}"

if [[ -f "${NGINX_PID_FILE}" ]]; then
  nginx_pid="$(cat "${NGINX_PID_FILE}")"
  if [[ -z "${nginx_pid}" ]]; then
    echo "invalid nginx pid file: ${NGINX_PID_FILE}"
  elif kill -0 "${nginx_pid}" 2>/dev/null; then
    kill "${nginx_pid}"
    echo "stopped frontend nginx pid=${nginx_pid}"
  else
    echo "frontend nginx already stopped pid=${nginx_pid}"
  fi
  rm -f "${NGINX_PID_FILE}"
else
  echo "frontend nginx is not running (pid file missing: ${NGINX_PID_FILE})"
fi

if [[ -f "${PID_FILE}" ]]; then
  backend_pid="$(cat "${PID_FILE}")"
  if [[ -z "${backend_pid}" ]]; then
    echo "invalid backend pid file: ${PID_FILE}"
    rm -f "${PID_FILE}"
    exit 1
  fi

  if kill -0 "${backend_pid}" 2>/dev/null; then
    kill "${backend_pid}"
    echo "stopped backend pid=${backend_pid}"
  else
    echo "backend already stopped pid=${backend_pid}"
  fi

  rm -f "${PID_FILE}"
else
  echo "backend is not running (pid file missing: ${PID_FILE})"
fi
