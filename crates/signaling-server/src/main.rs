// crates/signaling-server/src/main.rs
// Remote Desktop Signaling Server
// Runs on cloud VPS on port 443 (TLS)
// Responsibilities:
//   1. Device registration (Host registers with PIN)
//   2. Connection brokering (Client finds Host by PIN)
//   3. ICE candidate exchange (for P2P NAT traversal)
//   4. TURN relay fallback (when P2P fails)

mod registry;
mod relay;
mod stun;

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use registry::{DeviceRegistry, DeviceRole};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

/// Shared application state
pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub registry: DeviceRegistry,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            registry: DeviceRegistry::new(),
        }
    }
}

/// Messages sent over WebSocket between devices and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SignalingMessage {
    // ── Host → Server ──────────────────────────────────────────────────
    /// Host registers and announces its PIN
    HostRegister {
        pin: String,
        public_key: String, // Base64-encoded X25519 public key
    },

    // ── Client → Server ────────────────────────────────────────────────
    /// Client wants to connect to a host by PIN
    ClientConnect {
        pin: String,
        public_key: String, // Client's X25519 public key
    },

    // ── Server → Host ──────────────────────────────────────────────────
    /// Server tells host that a client wants to connect
    IncomingConnection {
        session_id: String,
        client_public_key: String,
        client_ip: String,
    },
    /// Host accepts the connection
    HostAccept { session_id: String },
    /// Host rejects the connection
    HostReject { session_id: String, reason: String },

    // ── Server → Client ────────────────────────────────────────────────
    /// Server tells client the host's details
    ConnectionAccepted {
        session_id: String,
        host_public_key: String,
    },
    ConnectionRejected { reason: String },

    // ── Both directions (ICE for P2P NAT traversal) ────────────────────
    IceCandidate {
        session_id: String,
        candidate: String,
        sdp_mid: Option<String>,
    },

    // ── Server → Any ───────────────────────────────────────────────────
    Error { message: String },
    Pong,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "signaling_server=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .with_state(state.clone());

    // Listen on 0.0.0.0:443 for TLS (or :8080 for dev)
    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;

    info!("Signaling server starting on {}", addr);

    // TODO: In production, use axum-server with TLS certs
    // For dev, run plain HTTP on :8080
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    info!("New WebSocket connection from {}", addr);
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

/// Handle a single WebSocket connection
async fn handle_socket(mut socket: WebSocket, addr: SocketAddr, state: SharedState) {
    let connection_id = Uuid::new_v4().to_string();
    info!("Connection {} established from {}", connection_id, addr);

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<SignalingMessage>(&text) {
                    Ok(signal) => {
                        if let Err(e) = handle_signal(signal, &connection_id, &addr, &state, &mut socket).await {
                            error!("Error handling signal: {}", e);
                            let err_msg = serde_json::to_string(&SignalingMessage::Error {
                                message: e.to_string(),
                            }).unwrap();
                            let _ = socket.send(Message::Text(err_msg.into())).await;
                        }
                    }
                    Err(e) => {
                        warn!("Invalid message from {}: {}", addr, e);
                    }
                }
            }
            Ok(Message::Ping(_)) => {
                let _ = socket.send(Message::Pong(vec![].into())).await;
            }
            Ok(Message::Close(_)) => {
                info!("Connection {} closed", connection_id);
                // Clean up registry
                state.registry.remove_device(&connection_id).await;
                break;
            }
            Err(e) => {
                error!("WebSocket error from {}: {}", addr, e);
                break;
            }
            _ => {}
        }
    }
}

/// Route incoming signaling messages
async fn handle_signal(
    signal: SignalingMessage,
    connection_id: &str,
    addr: &SocketAddr,
    state: &SharedState,
    socket: &mut WebSocket,
) -> Result<()> {
    match signal {
        // ── Host registers ────────────────────────────────────────────
        SignalingMessage::HostRegister { pin, public_key } => {
            info!("Host {} registered with PIN {}", connection_id, &pin[..3]);

            state.registry.register_host(
                connection_id.to_string(),
                pin.clone(),
                public_key,
                addr.ip().to_string(),
            ).await?;

            let response = serde_json::to_string(&SignalingMessage::Pong)?;
            socket.send(axum::extract::ws::Message::Text(response.into())).await?;
        }

        // ── Client wants to connect ───────────────────────────────────
        SignalingMessage::ClientConnect { pin, public_key } => {
            info!("Client {} requesting connection to PIN {}", connection_id, &pin[..3]);

            match state.registry.find_host_by_pin(&pin).await {
                Some(host) => {
                    let session_id = Uuid::new_v4().to_string();

                    // Register this client session
                    state.registry.register_client_session(
                        session_id.clone(),
                        connection_id.to_string(),
                        host.connection_id.clone(),
                    ).await;

                    // Notify host about incoming connection
                    let host_msg = serde_json::to_string(&SignalingMessage::IncomingConnection {
                        session_id: session_id.clone(),
                        client_public_key: public_key,
                        client_ip: addr.ip().to_string(),
                    })?;

                    state.registry.send_to_host(&host.connection_id, host_msg).await?;
                }
                None => {
                    let response = serde_json::to_string(&SignalingMessage::ConnectionRejected {
                        reason: "Invalid PIN or host offline".to_string(),
                    })?;
                    socket.send(axum::extract::ws::Message::Text(response.into())).await?;
                }
            }
        }

        // ── ICE candidate relay ───────────────────────────────────────
        SignalingMessage::IceCandidate { session_id, candidate, sdp_mid } => {
            // Forward ICE candidate to the other peer in this session
            state.registry.relay_ice_candidate(
                &session_id,
                connection_id,
                candidate,
                sdp_mid,
            ).await?;
        }

        _ => {
            warn!("Unexpected message type from {}", addr);
        }
    }

    Ok(())
}

