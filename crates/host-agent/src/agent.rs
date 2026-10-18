// crates/host-agent/src/agent.rs
// Main agent logic — connects to signaling server, manages sessions

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
use x25519_dalek::EphemeralSecret;

use crate::pin::PinManager;
use crate::session::run_session;

const SIGNALING_SERVER: &str = "wss://your-server.com/ws"; // Replace with your domain
const RECONNECT_DELAY_SECS: u64 = 5;

pub async fn run() -> Result<()> {
    let pin_manager = PinManager::load_or_generate();
    let pin = pin_manager.current_pin_display().to_string();
    info!("Persistent Device PIN active: {}", pin);

    // Show PIN to user via system notification on agent startup
    show_notification(
        "Remote Desktop",
        &format!("Device PIN: {} (Ready for connection)", pin),
    );

    // Connect to signaling server with resilient reconnect loop
    loop {
        info!("Host Agent listening with PIN: {}. Ready for connections...", pin);
        match connect_and_serve(&pin_manager).await {
            Ok(_) => {
                info!("Session disconnected. Re-registering with persistent PIN {} for next connection...", pin);
            }
            Err(e) => {
                error!("Connection/Signaling error: {} — reconnecting in {}s with same PIN", e, RECONNECT_DELAY_SECS);
                tokio::time::sleep(tokio::time::Duration::from_secs(RECONNECT_DELAY_SECS)).await;
            }
        }
        // PIN persists! Does NOT change when connection drops or session ends.
    }
}

async fn connect_and_serve(pin_manager: &PinManager) -> Result<()> {
    // Generate E2EE keypair
    let (our_secret, our_pub_key) = remote_desktop_core::E2EESession::generate_keypair();
    let pub_key_b64 = base64_encode(&our_pub_key);

    let pin = pin_manager.current_pin_display().to_string();

    // Connect to signaling server
    let (mut ws_stream, _) = connect_async(SIGNALING_SERVER).await?;
    info!("Connected to signaling server");

    // Register as host with our PIN and public key
    let register_msg = serde_json::json!({
        "type": "HostRegister",
        "data": {
            "pin": pin.replace('-', ""),
            "public_key": pub_key_b64
        }
    });
    ws_stream.send(Message::Text(register_msg.to_string().into())).await?;

    // Wait for incoming connection
    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let signal: serde_json::Value = serde_json::from_str(&text)?;

            match signal["type"].as_str() {
                Some("IncomingConnection") => {
                    let session_id = signal["data"]["session_id"].as_str().unwrap_or("").to_string();
                    let client_pub_key_b64 = signal["data"]["client_public_key"].as_str().unwrap_or("");
                    let client_ip = signal["data"]["client_ip"].as_str().unwrap_or("unknown");

                    info!("Incoming connection from {} (session: {})", client_ip, &session_id[..8]);

                    // TODO: Show approval dialog to user (for now, auto-accept)
                    // In production: show system dialog "Allow connection from X?"

                    // Complete E2EE handshake
                    let client_pub_bytes = base64_decode(client_pub_key_b64)?;
                    let mut client_pub = [0u8; 32];
                    client_pub.copy_from_slice(&client_pub_bytes);

                    // Run the remote session
                    info!("Starting remote session {}", &session_id[..8]);
                    let peer_addr: std::net::SocketAddr = client_ip.parse()
                        .unwrap_or_else(|_| "127.0.0.1:8080".parse().unwrap());
                    if let Err(e) = run_session(session_id, our_secret, client_pub, peer_addr).await {
                        error!("Session error: {}", e);
                    }
                    break;
                }
                Some("Error") => {
                    warn!("Server error: {}", signal["data"]["message"]);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn show_notification(title: &str, body: &str) {
    // macOS: use osascript
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"display notification "{}" with title "{}""#,
            body.replace('"', "'"),
            title.replace('"', "'")
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .ok();
    }

    // Windows: use PowerShell notification toast
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
            [System.Windows.Forms.MessageBox]::Show('{}', '{}')"#,
            body, title
        );
        std::process::Command::new("powershell")
            .arg("-Command")
            .arg(&script)
            .spawn()
            .ok();
    }

    info!("Notification: {} — {}", title, body);
}

fn base64_encode(data: &[u8]) -> String {
    use base64::prelude::*;
    BASE64_STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::prelude::*;
    BASE64_STANDARD.decode(s).map_err(|e| anyhow::anyhow!("Base64 decode error: {}", e))
}

