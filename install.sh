#!/usr/bin/env bash
set -euo pipefail

# Constants
SERVICE_NAME="claude-code-openai-wrapper"
BINARY_NAME="claude-code-openai-wrapper"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/claude-wrapper"
ENV_FILE="$CONFIG_DIR/config.env"
UNIT_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
SERVICE_USER="claude-wrapper"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HEALTH_URL="http://localhost:8000/health"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_err()   { echo -e "${RED}[ERROR]${NC} $*"; }

die() { log_err "$*"; exit 1; }

# ─── Uninstall ────────────────────────────────────────────────────────────────

do_uninstall() {
    log_info "Uninstalling ${SERVICE_NAME}..."

    if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        systemctl stop "$SERVICE_NAME"
        log_ok "Service stopped"
    fi

    if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        systemctl disable "$SERVICE_NAME"
        log_ok "Service disabled"
    fi

    if [[ -f "$UNIT_FILE" ]]; then
        rm -f "$UNIT_FILE"
        systemctl daemon-reload
        log_ok "Unit file removed"
    fi

    if [[ -f "$INSTALL_DIR/$BINARY_NAME" ]]; then
        rm -f "$INSTALL_DIR/$BINARY_NAME"
        log_ok "Binary removed"
    fi

    if [[ -d "$CONFIG_DIR" ]]; then
        rm -rf "$CONFIG_DIR"
        log_ok "Config directory removed"
    fi

    if id "$SERVICE_USER" &>/dev/null; then
        userdel "$SERVICE_USER"
        log_ok "Service user removed"
    fi

    log_ok "Uninstall complete"
    exit 0
}

# ─── Main ─────────────────────────────────────────────────────────────────────

if [[ "${1:-}" == "--uninstall" ]]; then
    [[ $EUID -eq 0 ]] || die "Must run as root: sudo $0 --uninstall"
    do_uninstall
fi

# Step 1: Preflight checks
log_info "Step 1/8: Preflight checks"

[[ $EUID -eq 0 ]] || die "Must run as root: sudo $0"

command -v cargo &>/dev/null || die "cargo not found. Install Rust: https://rustup.rs"

CLAUDE_CLI_PATH="$(command -v claude)" || die "claude CLI not found. Install it first."
log_ok "claude CLI found at $CLAUDE_CLI_PATH"

# Detect the real user who invoked sudo
REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME=$(getent passwd "$REAL_USER" | cut -d: -f6)
REAL_GROUP=$(id -gn "$REAL_USER")
log_ok "Real user: $REAL_USER (home: $REAL_HOME)"

# Step 2: Build dependencies
log_info "Step 2/8: Build dependencies"

MISSING_PKGS=()
dpkg -s pkg-config &>/dev/null || MISSING_PKGS+=(pkg-config)
dpkg -s libssl-dev &>/dev/null || MISSING_PKGS+=(libssl-dev)

if [[ ${#MISSING_PKGS[@]} -gt 0 ]]; then
    log_info "Installing: ${MISSING_PKGS[*]}"
    apt-get update -qq
    apt-get install -y -qq "${MISSING_PKGS[@]}"
    log_ok "Dependencies installed"
else
    log_ok "All dependencies present"
fi

# Step 3: Build
log_info "Step 3/8: Building release binary (this may take a few minutes)..."

cd "$SCRIPT_DIR"
sudo -u "$REAL_USER" cargo build --release 2>&1

BINARY_PATH="$SCRIPT_DIR/target/release/$BINARY_NAME"
[[ -f "$BINARY_PATH" ]] || die "Build failed: binary not found at $BINARY_PATH"
log_ok "Binary built ($(du -h "$BINARY_PATH" | cut -f1))"

# Step 4: Install binary
log_info "Step 4/8: Installing binary"

install -m 755 "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
log_ok "Installed to $INSTALL_DIR/$BINARY_NAME"

# Step 5: Service user
log_info "Step 5/8: Service user"

if id "$SERVICE_USER" &>/dev/null; then
    log_ok "User $SERVICE_USER already exists"
else
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
    log_ok "Created system user $SERVICE_USER"
fi

# Add service user to real user's group for ~/.claude access
if id -nG "$SERVICE_USER" | grep -qw "$REAL_GROUP"; then
    log_ok "$SERVICE_USER already in group $REAL_GROUP"
else
    usermod -aG "$REAL_GROUP" "$SERVICE_USER"
    log_ok "Added $SERVICE_USER to group $REAL_GROUP"
fi

# Step 6: Config file
log_info "Step 6/8: Config file"

mkdir -p "$CONFIG_DIR"

if [[ -f "$ENV_FILE" ]]; then
    log_warn "Config file exists, not overwriting: $ENV_FILE"
else
    cat > "$ENV_FILE" << 'ENVEOF'
# Claude Code OpenAI Wrapper Configuration
# See: https://github.com/anthropics/claude-code-openai-wrapper

# ─── Claude CLI ───────────────────────────────────────────────────────────────
# Full path to the claude CLI binary
CLAUDE_CLI_PATH=__CLAUDE_CLI_PATH__

# ─── Authentication ───────────────────────────────────────────────────────────
# Auth method: cli, api_key, bedrock, vertex
# If not set, auto-detects based on available env vars
# CLAUDE_AUTH_METHOD=cli

# Optional API key to protect the wrapper itself
# If not set, the server starts without API key protection
# API_KEY=your-optional-api-key-here

# ─── Server ───────────────────────────────────────────────────────────────────
PORT=8000
# CLAUDE_WRAPPER_HOST=0.0.0.0
# MAX_REQUEST_SIZE=10485760

# ─── Timeouts ─────────────────────────────────────────────────────────────────
# Wrapper-side timeout for waiting on CLI response (default: 600000 = 10 min)
MAX_TIMEOUT=600000

# ─── CORS ─────────────────────────────────────────────────────────────────────
CORS_ORIGINS=["*"]

# ─── Model ────────────────────────────────────────────────────────────────────
DEFAULT_MODEL=claude-sonnet-4-5-20250929

# ─── Claude CLI Subprocess ────────────────────────────────────────────────────
# These env vars are forwarded to the Claude CLI process
# CLAUDE_CODE_MAX_OUTPUT_TOKENS=128000
# BASH_DEFAULT_TIMEOUT_MS=120000
# BASH_MAX_TIMEOUT_MS=600000
# MAX_THINKING_TOKENS=32000
# CLAUDE_CLI_MAX_TURNS=0

# ─── Rate Limiting ────────────────────────────────────────────────────────────
RATE_LIMIT_ENABLED=true
RATE_LIMIT_PER_MINUTE=30
RATE_LIMIT_CHAT_PER_MINUTE=10
RATE_LIMIT_DEBUG_PER_MINUTE=2
RATE_LIMIT_AUTH_PER_MINUTE=10
RATE_LIMIT_SESSION_PER_MINUTE=15
RATE_LIMIT_HEALTH_PER_MINUTE=30
ENVEOF

    # Substitute the actual claude CLI path
    sed -i "s|__CLAUDE_CLI_PATH__|${CLAUDE_CLI_PATH}|" "$ENV_FILE"
    log_ok "Config written to $ENV_FILE"
fi

chown root:"$SERVICE_USER" "$ENV_FILE"
chmod 640 "$ENV_FILE"
log_ok "Config permissions set (640 root:$SERVICE_USER)"

# Step 7: Systemd unit file
log_info "Step 7/8: Systemd unit file"

cat > "$UNIT_FILE" << EOF
[Unit]
Description=Claude Code OpenAI Wrapper
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_USER}
EnvironmentFile=${ENV_FILE}
ExecStart=${INSTALL_DIR}/${BINARY_NAME}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
LimitNOFILE=65535

# Allow reading real user's ~/.claude for CLI auth
Environment=HOME=${REAL_HOME}

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadOnlyPaths=/
ReadWritePaths=/tmp
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

log_ok "Unit file written to $UNIT_FILE"

# Step 8: Enable & start
log_info "Step 8/8: Enable and start service"

systemctl daemon-reload
systemctl enable "$SERVICE_NAME" --quiet
systemctl restart "$SERVICE_NAME"
log_ok "Service enabled and started"

# Health check
log_info "Waiting for service to be ready..."
sleep 2

if curl -sf "$HEALTH_URL" >/dev/null 2>&1; then
    log_ok "Health check passed"
else
    log_warn "Health check failed — service may still be starting"
    log_warn "Check logs: journalctl -u $SERVICE_NAME -f"
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  ${SERVICE_NAME} installed successfully${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Binary:   ${INSTALL_DIR}/${BINARY_NAME}"
echo -e "  Config:   ${ENV_FILE}"
echo -e "  Unit:     ${UNIT_FILE}"
echo -e "  User:     ${SERVICE_USER}"
echo -e "  Port:     8000"
echo ""
echo -e "  ${BLUE}Useful commands:${NC}"
echo -e "    systemctl status  ${SERVICE_NAME}"
echo -e "    systemctl restart ${SERVICE_NAME}"
echo -e "    journalctl -u ${SERVICE_NAME} -f"
echo -e "    curl http://localhost:8000/health"
echo -e "    curl http://localhost:8000/v1/models"
echo ""
echo -e "  ${BLUE}Edit config:${NC}  sudo nano ${ENV_FILE}"
echo -e "  ${BLUE}Uninstall:${NC}    sudo $0 --uninstall"
echo ""
