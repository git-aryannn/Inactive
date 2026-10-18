// crates/host-agent/src/tray.rs
// System tray / menu bar icon for the host agent
// Shows PIN and allows user to:
//   - See current PIN
//   - Generate new PIN
//   - Disconnect active session
//   - Exit agent

// Note: Full system tray support will be added in a later phase
// using `tray-icon` crate (cross-platform)

/// Placeholder — tray icon integration to be implemented
pub struct TrayIcon;

impl TrayIcon {
    pub fn new() -> Self {
        Self
    }

    pub fn update_pin(&self, pin: &str) {
        // TODO: Update tray tooltip with current PIN
        tracing::info!("Tray: PIN updated to {}", pin);
    }

    pub fn show_connected(&self, peer_ip: &str) {
        tracing::info!("Tray: Connected to {}", peer_ip);
    }

    pub fn show_disconnected(&self) {
        tracing::info!("Tray: Disconnected");
    }
}

