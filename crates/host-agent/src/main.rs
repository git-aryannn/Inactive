// crates/host-agent/src/main.rs
// Remote Desktop Host Agent (Inactive)
// Runs as a resilient background host daemon on Windows and macOS.

mod agent;
mod pin;
mod session;
mod tray;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
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

    info!("Inactive Host Agent starting...");

    // Run the main agent loop
    agent::run().await
}
