#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/package/common.sh
source "${SCRIPT_DIR}/common.sh"

DEFAULT_PACKAGE_NAME="enginqa-rust-backend"
TARGET_OS=""
TARGET_ARCH=""
VERSION_TAG=""
OUTPUT_DIR="${PACKAGE_ROOT_DIR}/dist"
PACKAGE_NAME="${DEFAULT_PACKAGE_NAME}"

usage() {
  cat <<'EOF'
Usage:
  scripts/package/build-rust-backend.sh [options]

Options:
  --os <linux|windows>         Target operating system.
  --arch <x86_64|arm64>        Target architecture.
  --version <value>            Package version tag (default: date YYYYMMDD).
  --output-dir <path>          Output directory for packaged archive (default: ./dist).
  --package-name <value>       Package/binary name (default: enginqa-rust-backend).
  --help                       Show this message.

Examples:
  scripts/package/build-rust-backend.sh
  scripts/package/build-rust-backend.sh --os linux --arch arm64 --version v0.1.0
  scripts/package/build-rust-backend.sh --os windows --arch x86_64 --package-name engineqa-rust-backend
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

RUST_TARGET="$(rust_target_triple "${TARGET_OS}" "${TARGET_ARCH}" || true)"
[[ -n "${RUST_TARGET}" ]] || die "unsupported rust target combination: ${TARGET_OS}/${TARGET_ARCH}"

ARCHIVE_EXT="$(archive_extension_for_os "${TARGET_OS}")"
BIN_EXT="$(binary_extension_for_os "${TARGET_OS}")"
STAGE_NAME="${PACKAGE_NAME}-${VERSION_TAG}-${TARGET_OS}-${TARGET_ARCH}"
STAGE_DIR="${PACKAGE_BUILD_DIR}/${STAGE_NAME}"
ARCHIVE_PATH="${OUTPUT_DIR}/${STAGE_NAME}.${ARCHIVE_EXT}"
SOURCE_BINARY="${PACKAGE_ROOT_DIR}/backend/target/${RUST_TARGET}/release/engineqa-backend${BIN_EXT}"
TARGET_BINARY_REL="bin/${PACKAGE_NAME}${BIN_EXT}"

require_command cargo
mkdir -p "${PACKAGE_BUILD_DIR}" "${OUTPUT_DIR}"

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx "${RUST_TARGET}"; then
    die "rust target ${RUST_TARGET} is not installed. run: rustup target add ${RUST_TARGET}"
  fi
fi

log "building rust backend: target=${RUST_TARGET}"
cargo build --manifest-path "${PACKAGE_ROOT_DIR}/backend/Cargo.toml" --release --target "${RUST_TARGET}"

[[ -f "${SOURCE_BINARY}" ]] || die "compiled binary not found: ${SOURCE_BINARY}"

rm -rf "${STAGE_DIR}"
copy_common_assets "${STAGE_DIR}"
build_frontend_assets "${STAGE_DIR}"
copy_runtime_scripts "${STAGE_DIR}" "${TARGET_OS}"
write_runtime_env "${STAGE_DIR}" "rust" "${PACKAGE_NAME}" "${TARGET_BINARY_REL}"
write_runtime_readme "${STAGE_DIR}" "${PACKAGE_NAME}" "${TARGET_OS}" "${TARGET_ARCH}"

cp "${SOURCE_BINARY}" "${STAGE_DIR}/${TARGET_BINARY_REL}"
if [[ "${TARGET_OS}" == "linux" ]]; then
  chmod +x "${STAGE_DIR}/${TARGET_BINARY_REL}"
fi

create_archive "${TARGET_OS}" "${STAGE_DIR}" "${ARCHIVE_PATH}"
log "rust package created (backend + frontend): ${ARCHIVE_PATH}"
