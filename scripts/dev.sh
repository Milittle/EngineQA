#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_RUNTIME="${BACKEND_RUNTIME:-python}"

case "${BACKEND_RUNTIME}" in
  rust)
    exec "${ROOT_DIR}/scripts/dev-rust.sh"
    ;;
  python)
    exec "${ROOT_DIR}/scripts/dev-python.sh"
    ;;
  *)
    echo "unsupported BACKEND_RUNTIME=${BACKEND_RUNTIME}. use rust or python"
    exit 1
    ;;
esac
