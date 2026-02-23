#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/package/common.sh
source "${SCRIPT_DIR}/common.sh"

DEFAULT_PACKAGE_NAME="engineqa-python-backend"
TARGET_OS=""
TARGET_ARCH=""
VERSION_TAG=""
OUTPUT_DIR="${PACKAGE_ROOT_DIR}/dist"
PACKAGE_NAME="${DEFAULT_PACKAGE_NAME}"
if [[ -n "${BACKEND_PYTHON_BIN:-}" ]]; then
  PYTHON_BIN="${BACKEND_PYTHON_BIN}"
elif [[ -x "${PACKAGE_ROOT_DIR}/.venv-backend-python/bin/python" ]]; then
  PYTHON_BIN="${PACKAGE_ROOT_DIR}/.venv-backend-python/bin/python"
else
  PYTHON_BIN="python3"
fi

usage() {
  cat <<'EOF'
Usage:
  scripts/package/build-python-backend.sh [options]

Options:
  --os <linux|windows>         Target operating system.
  --arch <x86_64|arm64>        Target architecture.
  --version <value>            Package version tag (default: date YYYYMMDD).
  --output-dir <path>          Output directory for packaged archive (default: ./dist).
  --package-name <value>       Package/executable name (default: engineqa-python-backend).
  --python-bin <path|name>     Python executable to use (default: BACKEND_PYTHON_BIN,
                               then ./.venv-backend-python/bin/python, otherwise python3).
  --help                       Show this message.

Notes:
  Python backend package uses PyInstaller and does not support reliable cross-platform builds.
  Build it on the same OS/ARCH as the target package.

Examples:
  scripts/package/build-python-backend.sh
  scripts/package/build-python-backend.sh --os linux --arch arm64 --python-bin python3
  scripts/package/build-python-backend.sh --os windows --arch x86_64 --python-bin python
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --os)
      TARGET_OS="${2:-}"
      shift 2
      ;;
    --arch)
      TARGET_ARCH="${2:-}"
      shift 2
      ;;
    --version)
      VERSION_TAG="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --package-name)
      PACKAGE_NAME="${2:-}"
      shift 2
      ;;
    --python-bin)
      PYTHON_BIN="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1"
      ;;
  esac
done

if [[ -z "${TARGET_OS}" ]]; then
  TARGET_OS="$(detect_host_os)"
else
  TARGET_OS="$(normalize_os "${TARGET_OS}" || true)"
  [[ -n "${TARGET_OS}" ]] || die "unsupported target os"
fi

if [[ -z "${TARGET_ARCH}" ]]; then
  TARGET_ARCH="$(detect_host_arch)"
else
  TARGET_ARCH="$(normalize_arch "${TARGET_ARCH}" || true)"
  [[ -n "${TARGET_ARCH}" ]] || die "unsupported target arch"
fi

if [[ -z "${VERSION_TAG}" ]]; then
  VERSION_TAG="$(date +%Y%m%d)"
fi

HOST_OS="$(detect_host_os)"
HOST_ARCH="$(detect_host_arch)"

if [[ "${TARGET_OS}" != "${HOST_OS}" || "${TARGET_ARCH}" != "${HOST_ARCH}" ]]; then
  die "python package must be built on matching host/target. host=${HOST_OS}/${HOST_ARCH}, target=${TARGET_OS}/${TARGET_ARCH}"
fi

if [[ "${PYTHON_BIN}" == */* ]]; then
  [[ -x "${PYTHON_BIN}" ]] || die "python executable not found: ${PYTHON_BIN}"
else
  require_command "${PYTHON_BIN}"
fi

ARCHIVE_EXT="$(archive_extension_for_os "${TARGET_OS}")"
BIN_EXT="$(binary_extension_for_os "${TARGET_OS}")"
STAGE_NAME="${PACKAGE_NAME}-${VERSION_TAG}-${TARGET_OS}-${TARGET_ARCH}"
STAGE_DIR="${PACKAGE_BUILD_DIR}/${STAGE_NAME}"
ARCHIVE_PATH="${OUTPUT_DIR}/${STAGE_NAME}.${ARCHIVE_EXT}"

PYI_DIST_DIR="${PACKAGE_BUILD_DIR}/pyinstaller/dist/${STAGE_NAME}"
PYI_WORK_DIR="${PACKAGE_BUILD_DIR}/pyinstaller/work/${STAGE_NAME}"
PYI_SPEC_DIR="${PACKAGE_BUILD_DIR}/pyinstaller/spec/${STAGE_NAME}"
TARGET_BINARY_REL="bin/${PACKAGE_NAME}${BIN_EXT}"

mkdir -p "${PACKAGE_BUILD_DIR}" "${OUTPUT_DIR}" "${PYI_DIST_DIR}" "${PYI_WORK_DIR}" "${PYI_SPEC_DIR}"

log "checking python packaging dependencies"
if ! "${PYTHON_BIN}" -c "import fastapi,uvicorn,httpx,qdrant_client,dotenv" >/dev/null 2>&1; then
  die "python backend dependencies are missing. install: ${PYTHON_BIN} -m pip install -r backend-python/requirements.txt"
fi

if ! "${PYTHON_BIN}" -m PyInstaller --version >/dev/null 2>&1; then
  die "PyInstaller is missing. install: ${PYTHON_BIN} -m pip install pyinstaller"
fi

log "building python backend via pyinstaller"
"${PYTHON_BIN}" -m PyInstaller \
  --noconfirm \
  --clean \
  --onedir \
  --name "${PACKAGE_NAME}" \
  --paths "${PACKAGE_ROOT_DIR}/backend-python" \
  --collect-all qdrant_client \
  --collect-all dotenv \
  --distpath "${PYI_DIST_DIR}" \
  --workpath "${PYI_WORK_DIR}" \
  --specpath "${PYI_SPEC_DIR}" \
  "${PACKAGE_ROOT_DIR}/scripts/package/python/backend_entry.py"

PYI_APP_DIR="${PYI_DIST_DIR}/${PACKAGE_NAME}"
[[ -d "${PYI_APP_DIR}" ]] || die "pyinstaller output directory not found: ${PYI_APP_DIR}"

rm -rf "${STAGE_DIR}"
copy_common_assets "${STAGE_DIR}"
build_frontend_assets "${STAGE_DIR}"
copy_runtime_scripts "${STAGE_DIR}" "${TARGET_OS}"
write_runtime_env "${STAGE_DIR}" "python" "${PACKAGE_NAME}" "${TARGET_BINARY_REL}"
write_runtime_readme "${STAGE_DIR}" "${PACKAGE_NAME}" "${TARGET_OS}" "${TARGET_ARCH}"

cp -a "${PYI_APP_DIR}/." "${STAGE_DIR}/bin/"
if [[ "${TARGET_OS}" == "linux" ]]; then
  chmod +x "${STAGE_DIR}/${TARGET_BINARY_REL}"
fi

create_archive "${TARGET_OS}" "${STAGE_DIR}" "${ARCHIVE_PATH}"
log "python package created (backend + frontend): ${ARCHIVE_PATH}"
