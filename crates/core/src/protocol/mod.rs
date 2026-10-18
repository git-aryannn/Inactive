// crates/core/src/protocol/mod.rs
// Message protocol definitions — shared between host and client

use serde::{Deserialize, Serialize};

/// All message types exchanged over QUIC streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    // ── Handshake ──────────────────────────────────────
    /// Client sends PIN to signaling server
    ConnectRequest { pin: String, client_public_key: [u8; 32] },
    /// Signaling server forwards host's public key to client
    ConnectResponse { host_public_key: [u8; 32], session_id: String },
    /// ICE candidate for P2P NAT traversal
    IceCandidate { candidate: String, session_id: String },

    // ── Session Control ────────────────────────────────
    /// Host approves/rejects the connection
    SessionApprove { session_id: String },
    SessionReject { reason: String },
    /// Graceful disconnect
    Disconnect,
    /// Keepalive ping
    Ping,
    Pong,

    // ── Media ──────────────────────────────────────────
    /// Encoded video frame (H.264/H.265 NAL units)
    VideoFrame {
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp_ms: u64,
        keyframe: bool,
    },

    // ── Input Events ───────────────────────────────────
    /// Mouse movement (normalized 0.0-1.0 coordinates)
    MouseMove { x: f64, y: f64 },
    /// Mouse button event
    MouseButton { button: MouseButton, pressed: bool, x: f64, y: f64 },
    /// Mouse scroll
    MouseScroll { delta_x: f64, delta_y: f64 },
    /// Keyboard event
    KeyEvent { key: u32, pressed: bool, modifiers: u32 },

    // ── File Transfer ──────────────────────────────────
    /// Request to send a file
    FileTransferStart { file_name: String, file_size: u64, transfer_id: String },
    /// A chunk of file data
    FileChunk { transfer_id: String, offset: u64, data: Vec<u8> },
    /// File transfer complete
    FileTransferComplete { transfer_id: String },
    /// Transfer cancelled
    FileTransferCancel { transfer_id: String },

    // ── Clipboard ──────────────────────────────────────
    ClipboardSync { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Top-level message wrapper with E2EE encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: u64,
    /// Session identifier
    pub session_id: String,
    /// Encrypted payload (MessageType serialized then encrypted)
    pub payload: Vec<u8>,
    /// Nonce used for this message's encryption
    pub nonce: [u8; 12],
}

/// Session info displayed to user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_address: String,
    pub connected_at: u64,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Direct P2P connection (best, lowest latency)
    DirectP2P,
    /// Relayed through TURN server (fallback)
    Relayed,
    /// Local LAN (same network)
    LocalLAN,
}

