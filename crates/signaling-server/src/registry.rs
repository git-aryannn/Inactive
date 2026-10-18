// crates/signaling-server/src/registry.rs
// In-memory device registry — tracks connected hosts and active sessions

use anyhow::{Context, Result};
use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use crate::SignalingMessage;

/// A registered host device
#[derive(Debug)]
pub struct HostEntry {
    pub connection_id: String,
    pub pin: String,
    pub public_key: String,
    pub ip: String,
    /// Channel to send messages to this host's WebSocket
    pub tx: mpsc::Sender<String>,
}

/// An active session between a host and client
#[derive(Debug)]
pub struct SessionEntry {
    pub session_id: String,
    pub host_connection_id: String,
    pub client_connection_id: String,
}

pub enum DeviceRole {
    Host,
    Client,
}

/// Thread-safe device registry
pub struct DeviceRegistry {
    /// PIN → HostEntry
    hosts: Arc<RwLock<HashMap<String, HostEntry>>>,
    /// connection_id → PIN (for cleanup)
    connection_to_pin: Arc<RwLock<HashMap<String, String>>>,
    /// session_id → SessionEntry
    sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    /// connection_id → message sender (for WebSocket)
    senders: Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            connection_to_pin: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a host with its PIN
    pub async fn register_host(
        &self,
        connection_id: String,
        pin: String,
        public_key: String,
        ip: String,
    ) -> Result<()> {
        // Create a message channel for this host
        let (tx, _rx) = mpsc::channel::<String>(32);

        let entry = HostEntry {
            connection_id: connection_id.clone(),
            pin: pin.clone(),
            public_key,
            ip,
            tx: tx.clone(),
        };

        {
            let mut hosts = self.hosts.write().await;
            // Remove old registration if host re-registers
            hosts.remove(&pin);
            hosts.insert(pin.clone(), entry);
        }

        {
            let mut conn_map = self.connection_to_pin.write().await;
            conn_map.insert(connection_id.clone(), pin.clone());
        }

        {
            let mut senders = self.senders.write().await;
            senders.insert(connection_id, tx);
        }

        info!("Host registered with PIN {}", &pin[..3]);
        Ok(())
    }

    /// Find a host by PIN
    pub async fn find_host_by_pin(&self, pin: &str) -> Option<HostPublicInfo> {
        let hosts = self.hosts.read().await;
        hosts.get(pin).map(|h| HostPublicInfo {
            connection_id: h.connection_id.clone(),
            public_key: h.public_key.clone(),
        })
    }

    /// Register a session between host and client
    pub async fn register_client_session(
        &self,
        session_id: String,
        client_connection_id: String,
        host_connection_id: String,
    ) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), SessionEntry {
            session_id,
            host_connection_id,
            client_connection_id,
        });
    }

    /// Send a message to a specific host
    pub async fn send_to_host(&self, host_connection_id: &str, message: String) -> Result<()> {
        let senders = self.senders.read().await;
        if let Some(tx) = senders.get(host_connection_id) {
            tx.send(message).await.context("Failed to send to host")?;
        } else {
            warn!("Host {} not found in sender registry", host_connection_id);
        }
        Ok(())
    }

    /// Relay ICE candidate between peers in a session
    pub async fn relay_ice_candidate(
        &self,
        session_id: &str,
        from_connection_id: &str,
        candidate: String,
        sdp_mid: Option<String>,
    ) -> Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            // Find the OTHER peer
            let target_id = if from_connection_id == session.host_connection_id {
                &session.client_connection_id
            } else {
                &session.host_connection_id
            };

            let msg = serde_json::to_string(&SignalingMessage::IceCandidate {
                session_id: session_id.to_string(),
                candidate,
                sdp_mid,
            })?;

            self.send_to_connection(target_id, msg).await?;
        }
        Ok(())
    }

    /// Send message to any connection by ID
    pub async fn send_to_connection(&self, connection_id: &str, message: String) -> Result<()> {
        let senders = self.senders.read().await;
        if let Some(tx) = senders.get(connection_id) {
            tx.send(message).await.ok();
        }
        Ok(())
    }

    /// Remove device from registry (on disconnect)
    pub async fn remove_device(&self, connection_id: &str) {
        // Remove from senders
        self.senders.write().await.remove(connection_id);

        // Remove host entry if it was a host
        if let Some(pin) = self.connection_to_pin.write().await.remove(connection_id) {
            self.hosts.write().await.remove(&pin);
            info!("Host with PIN {} removed (disconnected)", &pin[..3.min(pin.len())]);
        }
    }
}

pub struct HostPublicInfo {
    pub connection_id: String,
    pub public_key: String,
}

