#!/usr/bin/env bash
# sdkwork-knowledgebase WSL standalone docker deployment.
#
# Runs inside WSL Ubuntu (invoked from Windows via `pnpm deploy:docker:wsl` or
# directly from a WSL shell). Orchestrates the full quick-deployment pipeline:
#   toolchain install -> workspace sync to ext4 -> cargo release build ->
#   portal dist build -> container image build -> docker compose up ->
#   nginx site + Windows hosts -> end-to-end verification.
#
# The workspace lives on the Windows filesystem (/mnt/<drive>/...); builds run
# on the WSL ext4 filesystem because 9p mounts are too slow for cargo/vite.
#
# Usage:
#   wsl-deploy.sh [options]
# Options:
#   --source <path>   Windows-side workspace root (default /mnt/e/sdkwork-space)
#   --target <path>   WSL ext4 workspace root (default $HOME/sdkwork-workspace)
#   --no-sync         skip workspace sync
#   --no-cargo        skip cargo release build
#   --no-frontend     skip portal dist build
#   --no-image        skip container image build
#   --no-compose      skip docker compose up
#   --no-nginx        skip nginx site + hosts
#   --verify-only     only run verification
#   -h, --help        show this help
set -euo pipefail

SOURCE_ROOT="/mnt/e/sdkwork-space"
TARGET_ROOT="${HOME}/sdkwork-workspace"
SYNC=1 CARGO_BUILD=1 FRONTEND=1 IMAGE_BUILD=1 COMPOSE=1 NGINX=1 VERIFY_ONLY=0

usage() {
  sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) SOURCE_ROOT="$2"; shift 2 ;;
    --target) TARGET_ROOT="$2"; shift 2 ;;
    --no-sync) SYNC=0; shift ;;
    --no-cargo) CARGO_BUILD=0; shift ;;
    --no-frontend) FRONTEND=0; shift ;;
    --no-image) IMAGE_BUILD=0; shift ;;
    --no-compose) COMPOSE=0; shift ;;
    --no-nginx) NGINX=0; shift ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

APP_REPO="sdkwork-knowledgebase"
APP_DIR="${TARGET_ROOT}/${APP_REPO}"
PORTAL_DIR="${TARGET_ROOT}/${APP_REPO}/apps/sdkwork-knowledgebase-pc/dist"
NGINX_SITE_DIR="/etc/nginx/sites-enabled/sdkwork"
NGINX_SITE_FILE="testapikb-knowledgebase.conf"
PORTAL_DEPLOY_ROOT="/opt/sdkwork/knowledgebase/portal"
DOMAINS="testapikb.sdkwork.com testapikb.birdcoder.com testapikb.noaper.com testapikb.dtupay.com"
HOSTS_LINE="127.0.0.1 ${DOMAINS}"

# Repositories required for the cargo + vite builds are discovered
# dynamically: the seed repo's manifests (cargo path deps + pnpm workspace
# ../sdkwork-* references) define the sibling set, scanned from the synced
# target on every run so the closure self-heals.

log() { printf '\n[deploy] %s\n' "$*"; }

run_sudo() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

require_tools() {
  log "preflight: install required tools (docker, nginx, rsync, curl, tar)"
  local packages=()
  command -v rsync >/dev/null 2>&1 || packages+=(rsync)
  command -v curl  >/dev/null 2>&1 || packages+=(curl)
  command -v docker >/dev/null 2>&1 || packages+=(docker.io docker-compose-plugin)
  command -v nginx  >/dev/null 2>&1 || packages+=(nginx)
  if [[ ${#packages[@]} -gt 0 ]]; then
    run_sudo apt-get update -y
    run_sudo apt-get install -y --no-install-recommends "${packages[@]}"
  fi
  docker version >/dev/null 2>&1 || {
    log "starting docker daemon"
    run_sudo systemctl enable --now docker >/dev/null 2>&1 \
      || run_sudo service docker start >/dev/null 2>&1 \
      || { run_sudo dockerd >/tmp/sdkwork-dockerd.log 2>&1 & sleep 8; }
  }
  docker version >/dev/null 2>&1 \
    || { echo "[deploy] docker daemon did not start; check /tmp/sdkwork-dockerd.log" >&2; exit 1; }
  docker compose version >/dev/null 2>&1 \
    || { echo "[deploy] docker compose plugin is missing" >&2; exit 1; }
  echo "[deploy] docker: $(docker version --format '{{.Server.Version}}')"
  echo "[deploy] compose: $(docker compose version --short)"
}

sync_workspace() {
  log "sync workspace ${SOURCE_ROOT} -> ${TARGET_ROOT} (ext4)"
  mkdir -p "${TARGET_ROOT}"
  local excludes=(
    --exclude='target/' --exclude='node_modules/' --exclude='dist/'
    --exclude='.git/' --exclude='.pnpm-store/' --exclude='.cargo-target-agents-check/'
    --exclude='.tmp/' --exclude='*.zip' --exclude='*.log' --exclude='.venv/'
    --exclude='__pycache__/' --exclude='.sdkwork/' --exclude='.zcode/' --exclude='.tools/'
  )
  # Seed repo first: its manifests define the referenced sibling set.
  rsync -a --info=stats1 "${excludes[@]}" \
    "${SOURCE_ROOT}/${APP_REPO}/" "${TARGET_ROOT}/${APP_REPO}/" \
    | tail -n 1 | sed "s/^/[deploy] synced ${APP_REPO}: /" || true
  # Then sync every sibling repo referenced by cargo path deps or the pnpm
  # workspace (../sdkwork-*). Scanning the already-synced target keeps this
  # fast on ext4 and self-heals when the dependency closure grows.
  local refs repo
  refs="$(grep -rhoE '\.\./sdkwork-[a-z0-9-]+' --include=Cargo.toml --include=pnpm-workspace.yaml \
    "${TARGET_ROOT}" 2>/dev/null | sed 's|\.\./||' | sort -u)"
  for repo in ${refs}; do
    if [[ -d "${SOURCE_ROOT}/${repo}" ]]; then
      if [[ ! -d "${TARGET_ROOT}/${repo}" ]]; then
        echo "[deploy] synced ${repo} (new)"
      fi
      rsync -a "${excludes[@]}" "${SOURCE_ROOT}/${repo}/" "${TARGET_ROOT}/${repo}/" >/dev/null 2>&1 || true
    else
      echo "[deploy] warn: missing source repo ${SOURCE_ROOT}/${repo}" >&2
    fi
  done
  # Toolchain/utility repos that may not be referenced by manifests.
  for repo in sdkwork-database sdkwork-app-topology sdkwork-sdk-generator; do
    if [[ -d "${SOURCE_ROOT}/${repo}" && ! -d "${TARGET_ROOT}/${repo}" ]]; then
      rsync -a "${excludes[@]}" "${SOURCE_ROOT}/${repo}/" "${TARGET_ROOT}/${repo}/"
      echo "[deploy] synced ${repo} (utility)"
    fi
  done
}

ensure_cargo() {
  if ! command -v cargo >/dev/null 2>&1; then
    log "install rustup (minimal profile)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable
    # shellcheck disable=SC1091
    . "${HOME}/.cargo/env"
  fi
  cargo --version
  rustc --version
}

build_cargo_release() {
  log "cargo build --release (gateway + worker; first build takes a while)"
  (cd "${APP_DIR}" && cargo build --release \
    -p sdkwork-api-knowledgebase-standalone-gateway \
    -p sdkwork-knowledgebase-worker)
}

ensure_node_pnpm() {
  if ! command -v node >/dev/null 2>&1 \
    || [[ "$(node -p 'process.versions.node.split(".")[0]')" -lt 22 ]]; then
    log "install Node.js 22 (NodeSource)"
    curl -fsSL https://deb.nodesource.com/setup_22.x | run_sudo -E bash -
    run_sudo apt-get install -y nodejs
  fi
  # Ignore Windows-side pnpm (e.g. /mnt/c/...): it would invoke the Windows
  # node runtime. Install the Linux pnpm (corepack) instead.
  if ! command -v pnpm >/dev/null 2>&1 || command -v pnpm | grep -q '^/mnt/'; then
    run_sudo npm install -g corepack
    corepack enable
    hash -r
  fi
  node --version
  pnpm --version
}

build_frontend() {
  log "pnpm install (workspace)"
  (cd "${APP_DIR}" && pnpm install --no-frozen-lockfile)
  # Credential-entry bootstrap Access-Token for the static portal build:
  # vite --mode standalone.docker defines process.env.SDKWORK_ACCESS_TOKEN
  # (see vite.config.ts), so the browser can start the login flow. The token
  # is signed with the IAM tenant signing master secret configured in .env.
  local secret token
  secret="$(sed -n 's/^SDKWORK_IAM_TENANT_SIGNING_MASTER_SECRET=//p' "${APP_DIR}/.env" 2>/dev/null | sed "s/[\"']//g" || true)"
  secret="${secret:-sdkwork-knowledgebase-dev-signing-secret}"
  token="$(node "${APP_DIR}/scripts/generate-bootstrap-token.mjs" --secret "${secret}")"
  log "vite build --mode standalone.docker (bootstrap token injected)"
  (cd "${APP_DIR}" \
    && SDKWORK_ACCESS_TOKEN="${token}" \
      pnpm --dir apps/sdkwork-knowledgebase-pc exec vite build --mode standalone.docker)
  test -f "${PORTAL_DIR}/index.html" \
    || { echo "[deploy] portal dist missing index.html" >&2; exit 1; }
}

build_container_image() {
  log "build container image (pnpm build:container)"
  (cd "${APP_DIR}" && node scripts/build-knowledgebase-container.mjs)
}

compose_up() {
  log "docker compose up -d"
  (cd "${APP_DIR}" \
    && [[ -f .env ]] || cp docker/.env.example .env)
  (cd "${APP_DIR}" && docker compose up -d)
  echo "[deploy] waiting for api readiness..."
  for _ in $(seq 1 60); do
    if curl -fsS http://127.0.0.1:3904/readyz >/dev/null 2>&1; then
      echo "[deploy] api ready"
      break
    fi
    sleep 5
  done
}

install_nginx() {
  log "install nginx site + portal static files"
  run_sudo mkdir -p "${NGINX_SITE_DIR}" "${PORTAL_DEPLOY_ROOT}"
  run_sudo rsync -a --delete "${PORTAL_DIR}/" "${PORTAL_DEPLOY_ROOT}/"
  run_sudo cp "${APP_DIR}/docker/nginx/${NGINX_SITE_FILE}" "${NGINX_SITE_DIR}/"
  # The testapidocker.* domains may be claimed by a previous app deployment
  # (e.g. sdkwork-im's testapidocker-im.conf). Disable it non-destructively
  # (.orig, matching the existing rotation convention) so the knowledgebase
  # site owns the domains.
  if [[ -f /etc/nginx/sites-enabled/testapidocker-im.conf ]]; then
    run_sudo mv /etc/nginx/sites-enabled/testapidocker-im.conf \
      /etc/nginx/sites-enabled/testapidocker-im.conf.orig
    echo "[deploy] disabled testapidocker-im.conf (.orig); restore by renaming back"
  fi
  run_sudo nginx -t
  run_sudo systemctl reload nginx 2>/dev/null || run_sudo service nginx reload
  echo "[deploy] nginx site enabled: ${NGINX_SITE_DIR}/${NGINX_SITE_FILE}"
}

update_hosts() {
  local windows_hosts="/mnt/c/Windows/System32/drivers/etc/hosts"
  log "update Windows hosts: ${HOSTS_LINE}"
  if [[ -w "${windows_hosts}" ]]; then
    if ! grep -q "testapikb.sdkwork.com" "${windows_hosts}" 2>/dev/null; then
      printf '\n# sdkwork-knowledgebase docker test domains\n%s\n' "${HOSTS_LINE}" \
        | run_sudo tee -a "${windows_hosts}" >/dev/null
    fi
    echo "[deploy] Windows hosts updated"
  else
    cat <<'EOF'
[deploy] cannot write C:\Windows\System32\drivers\etc\hosts from WSL.
Run this once in an elevated (Administrator) PowerShell on Windows:

  Add-Content -Path C:\Windows\System32\drivers\etc\hosts -Value "`n# sdkwork-knowledgebase docker test domains`n127.0.0.1 testapikb.sdkwork.com testapikb.birdcoder.com testapikb.noaper.com testapikb.dtupay.com"

EOF
  fi
  # WSL-local /etc/hosts so WSL-side curl checks work (entries are harmless
  # even when WSL regenerates the file).
  if ! grep -q "testapikb.sdkwork.com" /etc/hosts 2>/dev/null; then
    printf '\n%s\n' "${HOSTS_LINE}" | run_sudo tee -a /etc/hosts >/dev/null
  fi
}

verify() {
  log "verification"
  echo "--- docker compose ps ---"
  (cd "${APP_DIR}" && docker compose ps)
  echo "--- gateway probes (127.0.0.1:3904) ---"
  for probe in healthz readyz livez; do
    printf '%-10s %s\n' "${probe}" "$(curl -fsS -w ' [%{http_code}]' "http://127.0.0.1:3904/${probe}" 2>/dev/null || echo FAIL)"
  done
  printf '%-10s %s\n' "openapi.json" "$(curl -fsS -o /dev/null -w '[%{http_code}]' http://127.0.0.1:3904/openapi.json 2>/dev/null || echo FAIL)"
  echo "--- nginx portal (Host: testapikb.sdkwork.com) ---"
  printf '%-10s %s\n' "/" "$(curl -fsS -H 'Host: testapikb.sdkwork.com' -o /dev/null -w '[%{http_code}]' http://127.0.0.1/ 2>/dev/null || echo FAIL)"
  printf '%-10s %s\n' "readyz" "$(curl -fsS -H 'Host: testapikb.sdkwork.com' -o /dev/null -w '[%{http_code}]' http://127.0.0.1/readyz 2>/dev/null || echo FAIL)"
  echo "--- api plane smoke ---"
  printf '%-10s %s\n' "openapi" "$(curl -fsS -H 'Host: testapikb.sdkwork.com' -o /dev/null -w '[%{http_code}]' http://127.0.0.1/openapi.json 2>/dev/null || echo FAIL)"
  echo "[deploy] done. Visit http://testapikb.sdkwork.com (or birdcoder/dtupay) from Windows."
}

if [[ "${VERIFY_ONLY}" -eq 1 ]]; then
  verify
  exit 0
fi

require_tools
[[ "${SYNC}" -eq 1 ]] && sync_workspace
[[ "${CARGO_BUILD}" -eq 1 ]] && { ensure_cargo; build_cargo_release; }
[[ "${FRONTEND}" -eq 1 ]] && { ensure_node_pnpm; build_frontend; }
[[ "${IMAGE_BUILD}" -eq 1 ]] && build_container_image
[[ "${COMPOSE}" -eq 1 ]] && compose_up
[[ "${NGINX}" -eq 1 ]] && { install_nginx; update_hosts; }
verify
