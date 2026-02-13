#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_PORT="${APP_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-5173}"
DEFAULT_VENV_PYTHON="${ROOT_DIR}/.venv-backend-python/bin/python"
PYDEPS_DIR="${ROOT_DIR}/backend-python/.pydeps"
SELECTED_PYTHON_BIN=""
SELECTED_PYTHONPATH=""

if [[ ! -d "${ROOT_DIR}/frontend/node_modules" ]]; then
  echo "frontend dependencies are missing. run: npm install --prefix frontend"
  exit 1
fi

resolve_python_bin() {
  local candidate="$1"

  if [[ "${candidate}" == */* ]]; then
    if [[ -x "${candidate}" ]]; then
      echo "${candidate}"
      return 0
    fi
    return 1
  fi

  command -v "${candidate}" 2>/dev/null || return 1
}

has_python_deps() {
  local python_bin="$1"
  local extra_pythonpath="${2:-}"

  if [[ -n "${extra_pythonpath}" ]]; then
    PYTHONPATH="${extra_pythonpath}${PYTHONPATH:+:${PYTHONPATH}}" \
      "${python_bin}" -c "import fastapi, uvicorn, httpx, qdrant_client, dotenv" >/dev/null 2>&1
    return $?
  fi

  "${python_bin}" -c "import fastapi, uvicorn, httpx, qdrant_client, dotenv" >/dev/null 2>&1
}

select_python_runtime() {
  local candidate
  local resolved

  if [[ -n "${BACKEND_PYTHON_BIN:-}" ]]; then
    resolved="$(resolve_python_bin "${BACKEND_PYTHON_BIN}" || true)"
    if [[ -z "${resolved}" ]]; then
      echo "python runtime not found: ${BACKEND_PYTHON_BIN}"
      return 1
    fi

    if has_python_deps "${resolved}"; then
      SELECTED_PYTHON_BIN="${resolved}"
      return 0
    fi

    if [[ -d "${PYDEPS_DIR}" ]] && has_python_deps "${resolved}" "${PYDEPS_DIR}"; then
      SELECTED_PYTHON_BIN="${resolved}"
      SELECTED_PYTHONPATH="${PYDEPS_DIR}"
      return 0
    fi

    echo "python dependencies are missing for backend-python"
    echo "install with: ${resolved} -m pip install -r backend-python/requirements.txt"
    echo "or fallback without venv:"
    echo "  python3 -m pip install --target backend-python/.pydeps -r backend-python/requirements.txt"
    return 1
  fi

  for candidate in "${DEFAULT_VENV_PYTHON}" python3 python; do
    resolved="$(resolve_python_bin "${candidate}" || true)"
    if [[ -z "${resolved}" ]]; then
      continue
    fi

    if has_python_deps "${resolved}"; then
      SELECTED_PYTHON_BIN="${resolved}"
      return 0
    fi

    if [[ -d "${PYDEPS_DIR}" ]] && has_python_deps "${resolved}" "${PYDEPS_DIR}"; then
      SELECTED_PYTHON_BIN="${resolved}"
      SELECTED_PYTHONPATH="${PYDEPS_DIR}"
      return 0
    fi
  done

  echo "python dependencies are missing for backend-python"
  echo "preferred setup:"
  echo "  python3 -m venv .venv-backend-python"
  echo "  .venv-backend-python/bin/pip install -r backend-python/requirements.txt"
  echo "fallback without venv:"
  echo "  python3 -m pip install --target backend-python/.pydeps -r backend-python/requirements.txt"
  return 1
}

if ! select_python_runtime; then
  exit 1
fi

if [[ -n "${SELECTED_PYTHONPATH}" ]]; then
  echo "using python runtime: ${SELECTED_PYTHON_BIN} (PYTHONPATH += backend-python/.pydeps)"
else
  echo "using python runtime: ${SELECTED_PYTHON_BIN}"
fi

backend_cmd() {
  if [[ -n "${SELECTED_PYTHONPATH}" ]]; then
    PYTHONPATH="${SELECTED_PYTHONPATH}${PYTHONPATH:+:${PYTHONPATH}}" \
      "${SELECTED_PYTHON_BIN}" -m uvicorn app.main:app --app-dir backend-python --host 0.0.0.0 --port "${BACKEND_PORT}"
    return
  fi

  "${SELECTED_PYTHON_BIN}" -m uvicorn app.main:app --app-dir backend-python --host 0.0.0.0 --port "${BACKEND_PORT}"
}

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
  backend_cmd
) &
backend_pid=$!

(
  cd "${ROOT_DIR}"
  npm run dev --prefix frontend -- --host 0.0.0.0 --port "${FRONTEND_PORT}"
) &
frontend_pid=$!

echo "backend(runtime=python, qdrant=${QDRANT_MODE:-embedded}) -> http://127.0.0.1:${BACKEND_PORT}/health"
echo "frontend                                             -> http://127.0.0.1:${FRONTEND_PORT}"
echo "press Ctrl+C to stop"

wait -n "${backend_pid}" "${frontend_pid}"
