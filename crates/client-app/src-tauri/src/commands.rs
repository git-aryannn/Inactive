// crates/client-app/src-tauri/src/commands.rs
// Tauri commands exposed to the React frontend

use anyhow::Result;
use remote_desktop_core::protocol::{MessageType, MouseButton};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Connection state shared across commands
pub struct ConnectionState {
    pub status: ConnectionStatus,
    pub session_id: Option<String>,
    pub peer_addr: Option<std::net::SocketAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected { peer_ip: String, connection_type: String },
    Error(String),
}

/// Connect to a host by PIN
#[tauri::command]
pub async fn connect_to_host(
    pin: String,
    window: tauri::Window,
) -> Result<String, String> {
    // Remove dashes, validate 9 digits
    let clean_pin: String = pin.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean_pin.len() != 9 {
        return Err("PIN must be 9 digits".to_string());
    }

    // TODO: Full connection flow:
    // 1. Generate E2EE keypair
    // 2. Connect to signaling server WebSocket
    // 3. Send ClientConnect with PIN + our public key
    // 4. Wait for ConnectionAccepted with host's public key
    // 5. Complete ECDH handshake → session key
    // 6. Exchange ICE candidates for P2P
    // 7. Connect QUIC P2P (or relay fallback)
    // 8. Start receiving video stream

    // Emit progress to frontend
    window.emit("connection_status", "Connecting...").ok();

    Ok("Connecting...".to_string())
}

/// Disconnect current session
#[tauri::command]
pub async fn disconnect() -> Result<(), String> {
    // TODO: Send Disconnect message and close QUIC connection
    Ok(())
}

/// Send mouse movement event to host
#[tauri::command]
pub async fn send_mouse_event(
    x: f64,
    y: f64,
    button: Option<String>,
    pressed: Option<bool>,
    scroll_delta: Option<f64>,
) -> Result<(), String> {
    // TODO: Serialize event, encrypt, send over QUIC control stream
    Ok(())
}

/// Send keyboard event to host
#[tauri::command]
pub async fn send_key_event(key_code: u32, pressed: bool) -> Result<(), String> {
    // TODO: Serialize event, encrypt, send over QUIC control stream
    Ok(())
}

/// Get current connection status
#[tauri::command]
pub async fn get_connection_status() -> Result<String, String> {
    Ok("disconnected".to_string())
}

