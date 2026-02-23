#!/usr/bin/env bash
set -euo pipefail

PACKAGE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_ROOT_DIR="$(cd "${PACKAGE_SCRIPT_DIR}/../.." && pwd)"
PACKAGE_BUILD_DIR="${PACKAGE_ROOT_DIR}/.package-build"

log() {
  printf '[package] %s\n' "$*"
}

die() {
  printf '[package][error] %s\n' "$*" >&2
  exit 1
}

require_command() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || die "command not found: ${command_name}"
}

normalize_os() {
  local raw="${1:-}"
  raw="${raw,,}"
  case "${raw}" in
    linux)
      echo "linux"
      ;;
    windows | win | win32 | mingw* | msys* | cygwin*)
      echo "windows"
      ;;
    *)
      return 1
      ;;
  esac
}

normalize_arch() {
  local raw="${1:-}"
  raw="${raw,,}"
  case "${raw}" in
    x86_64 | amd64)
      echo "x86_64"
      ;;
    arm64 | aarch64)
      echo "arm64"
      ;;
    *)
      return 1
      ;;
  esac
}

detect_host_os() {
  local host_uname
  host_uname="$(uname -s)"
  normalize_os "${host_uname}" || die "unsupported host os: ${host_uname}"
}

detect_host_arch() {
  local host_arch
  host_arch="$(uname -m)"
  normalize_arch "${host_arch}" || die "unsupported host arch: ${host_arch}"
}

rust_target_triple() {
  local target_os="$1"
  local target_arch="$2"
  case "${target_os}/${target_arch}" in
    linux/x86_64)
      echo "x86_64-unknown-linux-gnu"
      ;;
    linux/arm64)
      echo "aarch64-unknown-linux-gnu"
      ;;
    windows/x86_64)
      echo "x86_64-pc-windows-msvc"
      ;;
    windows/arm64)
      echo "aarch64-pc-windows-msvc"
      ;;
    *)
      return 1
      ;;
  esac
}

archive_extension_for_os() {
  local target_os="$1"
  case "${target_os}" in
    linux)
      echo "tar.gz"
      ;;
    windows)
      echo "zip"
      ;;
    *)
      return 1
      ;;
  esac
}

binary_extension_for_os() {
  local target_os="$1"
  case "${target_os}" in
    linux)
      echo ""
      ;;
    windows)
      echo ".exe"
      ;;
    *)
      return 1
      ;;
  esac
}

create_archive() {
  local target_os="$1"
  local stage_dir="$2"
  local output_path="$3"

  mkdir -p "$(dirname "${output_path}")"
  rm -f "${output_path}"

  case "${target_os}" in
    linux)
      tar -C "$(dirname "${stage_dir}")" -czf "${output_path}" "$(basename "${stage_dir}")"
      ;;
    windows)
      require_command zip
      (
        cd "$(dirname "${stage_dir}")"
        zip -qr "${output_path}" "$(basename "${stage_dir}")"
      )
      ;;
    *)
      die "unsupported archive os: ${target_os}"
      ;;
  esac
}

copy_common_assets() {
  local stage_dir="$1"

  mkdir -p \
    "${stage_dir}/bin" \
    "${stage_dir}/config" \
    "${stage_dir}/data" \
    "${stage_dir}/knowledge" \
    "${stage_dir}/logs" \
    "${stage_dir}/run"

  cp "${PACKAGE_ROOT_DIR}/.env.example" "${stage_dir}/config/.env.example"
  if [[ -d "${PACKAGE_ROOT_DIR}/knowledge" ]]; then
    cp -a "${PACKAGE_ROOT_DIR}/knowledge/." "${stage_dir}/knowledge/"
  fi
}

build_frontend_assets() {
  local stage_dir="$1"
  local frontend_root="${PACKAGE_ROOT_DIR}/frontend"
  local frontend_dist_dir="${frontend_root}/dist"
  local frontend_lock_file="${frontend_root}/package-lock.json"

  [[ -d "${frontend_root}" ]] || die "frontend directory not found: ${frontend_root}"
  [[ -f "${frontend_root}/package.json" ]] || die "frontend package manifest not found: ${frontend_root}/package.json"

  require_command npm
  if [[ ! -d "${frontend_root}/node_modules" ]]; then
    log "frontend dependencies missing, installing packages"
    if [[ -f "${frontend_lock_file}" ]]; then
      npm --prefix "${frontend_root}" ci
    else
      npm --prefix "${frontend_root}" install
    fi
  fi

  log "building frontend assets"
  npm --prefix "${frontend_root}" run build

  [[ -d "${frontend_dist_dir}" ]] || die "frontend build output not found: ${frontend_dist_dir}"
  rm -rf "${stage_dir}/frontend"
  mkdir -p "${stage_dir}/frontend"
  cp -a "${frontend_dist_dir}/." "${stage_dir}/frontend/"
}

copy_runtime_scripts() {
  local stage_dir="$1"
  local target_os="$2"

  case "${target_os}" in
    linux)
      cp "${PACKAGE_SCRIPT_DIR}/runtime/linux/start-backend.sh" "${stage_dir}/start.sh"
      cp "${PACKAGE_SCRIPT_DIR}/runtime/linux/stop-backend.sh" "${stage_dir}/stop.sh"
      chmod +x "${stage_dir}/start.sh" "${stage_dir}/stop.sh"
      ;;
    windows)
      cp "${PACKAGE_SCRIPT_DIR}/runtime/windows/start-backend.ps1" "${stage_dir}/start.ps1"
      cp "${PACKAGE_SCRIPT_DIR}/runtime/windows/stop-backend.ps1" "${stage_dir}/stop.ps1"
      ;;
    *)
      die "unsupported runtime script os: ${target_os}"
      ;;
  esac
}

write_runtime_env() {
  local stage_dir="$1"
  local runtime_kind="$2"
  local package_name="$3"
  local backend_bin_rel="$4"

  cat > "${stage_dir}/runtime.env" <<EOF
# Runtime metadata for launcher scripts.
ENGINEQA_PACKAGE_NAME=${package_name}
ENGINEQA_RUNTIME_KIND=${runtime_kind}
ENGINEQA_BACKEND_BIN=${backend_bin_rel}
ENGINEQA_FRONTEND_DIR=frontend
ENGINEQA_PID_FILE=run/backend.pid
ENGINEQA_LOG_FILE=logs/backend.log
ENGINEQA_NGINX_PID_FILE=run/nginx.pid
ENGINEQA_NGINX_CONF_FILE=run/nginx.conf
ENGINEQA_NGINX_ACCESS_LOG=logs/nginx.access.log
ENGINEQA_NGINX_ERROR_LOG=logs/nginx.error.log
EOF
}

write_runtime_readme() {
  local stage_dir="$1"
  local package_name="$2"
  local target_os="$3"
  local target_arch="$4"

  cat > "${stage_dir}/README-runtime.md" <<EOF
# ${package_name}

Target platform: ${target_os}/${target_arch}

## Quick Start
1. Copy config template:
   - Linux: \`cp config/.env.example .env\`
   - Windows: \`Copy-Item config/.env.example .env\`
2. Fill required env values in \`.env\`:
   - \`INTERNAL_API_BASE_URL\`
   - \`INTERNAL_API_TOKEN\`
3. Start services (backend + nginx frontend):
   - Linux: \`./start.sh\`
   - Windows: \`./start.ps1\`
4. Verify:
   - Backend: \`curl http://127.0.0.1:8080/health\`
   - Frontend: open \`http://127.0.0.1:5173\`
5. Stop services:
   - Linux: \`./stop.sh\`
   - Windows: \`./stop.ps1\`

## Notes
- This package contains backend runtime and prebuilt frontend assets (\`frontend/\`), served by nginx.
- nginx must be available on the target host and in PATH.
- Logs are written to \`logs/backend.log\`.
- Knowledge files are located in \`knowledge/\`.
EOF
}
