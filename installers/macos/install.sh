#!/usr/bin/env bash
# macOS Installation Script for Inactive Host Agent
# Runs as LaunchDaemon (Root level, persists across logouts, protected from termination)

set -e

echo "=== Inactive Agent — macOS Installer ==="

# Check root permissions
if [ "$EUID" -ne 0 ]; then
  echo "❌ Please run as root (sudo ./install.sh)"
  exit 1
fi

BINARY_NAME="inactive-agent"
INSTALL_BIN_DIR="/usr/local/bin"
PLIST_NAME="com.inactive.agent.plist"
PLIST_SRC="$(dirname "$0")/com.inactive.agent.plist"
PLIST_DEST="/Library/LaunchDaemons/${PLIST_NAME}"
LOG_DIR="/var/log/inactive"

echo "1. Creating log directory ${LOG_DIR}..."
mkdir -p "${LOG_DIR}"
chmod 755 "${LOG_DIR}"

# If prebuilt binary exists, install it
SRC_BIN=""
for candidate in \
  "$(dirname "$0")/../../target/release/inactive-agent" \
  "$(dirname "$0")/../../target/release/host-agent" \
  "$(dirname "$0")/../../target/debug/inactive-agent" \
  "$(dirname "$0")/../../target/debug/host-agent"; do
  if [ -f "$candidate" ]; then
    SRC_BIN="$candidate"
    break
  fi
done

if [ -n "$SRC_BIN" ]; then
  echo "2. Copying binary from ${SRC_BIN} to ${INSTALL_BIN_DIR}/${BINARY_NAME}..."
  cp "${SRC_BIN}" "${INSTALL_BIN_DIR}/${BINARY_NAME}"
else
  echo "⚠️ Binary not found. Building inactive-agent..."
  cargo build --release --package host-agent
  cp "$(dirname "$0")/../../target/release/inactive-agent" "${INSTALL_BIN_DIR}/${BINARY_NAME}" 2>/dev/null || \
  cp "$(dirname "$0")/../../target/release/host-agent" "${INSTALL_BIN_DIR}/${BINARY_NAME}"
fi

chmod 755 "${INSTALL_BIN_DIR}/${BINARY_NAME}"

echo "3. Installing LaunchDaemon plist to ${PLIST_DEST}..."
cp "${PLIST_SRC}" "${PLIST_DEST}"
chown root:wheel "${PLIST_DEST}"
chmod 644 "${PLIST_DEST}"

echo "4. Unloading existing daemon (if running)..."
launchctl unload "${PLIST_DEST}" 2>/dev/null || true

echo "5. Loading LaunchDaemon..."
launchctl load -w "${PLIST_DEST}"

echo "✅ Inactive Agent successfully installed and running!"
echo "   - Status check: sudo launchctl list | grep inactive"
echo "   - View logs:    tail -f ${LOG_DIR}/inactive-agent.log"
