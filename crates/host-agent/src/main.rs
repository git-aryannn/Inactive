// crates/host-agent/src/main.rs
// Remote Desktop Host Agent
// Runs as:
//   - Windows: SYSTEM-level Windows Service (via windows-service crate)
//   - macOS:   Root LaunchDaemon (via /Library/LaunchDaemons plist)
//
// Responsibilities:
//   1. Connect to signaling server and register with generated PIN
//   2. Wait for incoming connection requests
//   3. Show PIN to local user (via system notification or tray icon)
//   4. Accept/reject incoming connections
//   5. Capture screen + encode + stream to client
//   6. Receive and replay client input events

mod agent;
mod pin;
mod session;
mod tray;

#[cfg(target_os = "windows")]
mod service_windows;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(target_os = "windows")]
use windows_service::{define_windows_service, service_dispatcher};

#[cfg(target_os = "windows")]
define_windows_service!(ffi_service_main, windows_service_main);

#[cfg(target_os = "windows")]
fn windows_service_main(args: Vec<std::ffi::OsString>) {
    if let Err(e) = service_windows::run_service(args) {
        eprintln!("Service error: {}", e);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    // On Windows: log to Windows Event Log
    // On macOS: log to /var/log/inactive/inactive-agent.log
    let log_dir = if cfg!(target_os = "macos") {
        "/var/log/inactive".to_string()
    } else {
        dirs::data_dir()
            .unwrap_or_default()
            .join("Inactive")
            .join("logs")
            .to_string_lossy()
            .to_string()
    };

    std::fs::create_dir_all(&log_dir).ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "host_agent=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Check if we should run as Windows Service dispatcher
    #[cfg(target_os = "windows")]
    {
        if std::env::args().any(|a| a == "--service") {
            return service_dispatcher::start("InactiveAgent", ffi_service_main)
                .map_err(|e| anyhow::anyhow!("Service dispatch failed: {}", e));
        }
    }

    info!("Inactive Host Agent starting...");

    // Run the main agent loop
    agent::run().await
}

