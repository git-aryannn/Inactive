#!/usr/bin/env bash
# macOS Installation Script for Remote Desk Host Agent
# Runs as LaunchDaemon (Root level, persists across logouts, protected from termination)

set -e

echo "=== Remote Desk Agent — macOS Installer ==="

# Check root permissions
if [ "$EUID" -ne 0 ]; then
  echo "❌ Please run as root (sudo ./install.sh)"
  exit 1
fi

BINARY_NAME="remote-desk-agent"
INSTALL_BIN_DIR="/usr/local/bin"
PLIST_NAME="com.remotedesk.agent.plist"
PLIST_SRC="$(dirname "$0")/com.remotedesk.agent.plist"
PLIST_DEST="/Library/LaunchDaemons/${PLIST_NAME}"
LOG_DIR="/var/log/remote-desktop"

echo "1. Creating log directory ${LOG_DIR}..."
mkdir -p "${LOG_DIR}"
chmod 755 "${LOG_DIR}"

# If prebuilt binary exists, install it
if [ -f "$(dirname "$0")/../../target/release/host-agent" ]; then
  echo "2. Copying release binary to ${INSTALL_BIN_DIR}/${BINARY_NAME}..."
  cp "$(dirname "$0")/../../target/release/host-agent" "${INSTALL_BIN_DIR}/${BINARY_NAME}"
elif [ -f "$(dirname "$0")/../../target/debug/host-agent" ]; then
  echo "2. Copying debug binary to ${INSTALL_BIN_DIR}/${BINARY_NAME}..."
  cp "$(dirname "$0")/../../target/debug/host-agent" "${INSTALL_BIN_DIR}/${BINARY_NAME}"
else
  echo "⚠️ Binary not found. Building host-agent..."
  cargo build --release --package host-agent
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

echo "✅ Remote Desk Agent successfully installed and running!"
echo "   - Status check: sudo launchctl list | grep remotedesk"
echo "   - View logs:    tail -f ${LOG_DIR}/remote-desk-agent.log"

