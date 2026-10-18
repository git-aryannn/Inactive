// crates/signaling-server/src/relay.rs
// TURN-like relay fallback
// Used when P2P NAT traversal fails (symmetric NAT, corporate firewall)
// Video/control data is relayed through this server encrypted (E2EE)

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

/// Active relay session
pub struct RelaySession {
    pub session_id: String,
    pub host_tx: mpsc::Sender<Vec<u8>>,
    pub client_tx: mpsc::Sender<Vec<u8>>,
    pub bytes_relayed: u64,
    pub started_at: std::time::Instant,
}

/// TURN-like relay manager
pub struct RelayManager {
    sessions: Arc<RwLock<HashMap<String, RelaySession>>>,
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a relay tunnel between host and client
    pub async fn create_session(
        &self,
        session_id: String,
    ) -> Result<(mpsc::Receiver<Vec<u8>>, mpsc::Receiver<Vec<u8>>)> {
        let (host_tx, host_rx) = mpsc::channel(64);
        let (client_tx, client_rx) = mpsc::channel(64);

        let session = RelaySession {
            session_id: session_id.clone(),
            host_tx,
            client_tx,
            bytes_relayed: 0,
            started_at: std::time::Instant::now(),
        };

        self.sessions.write().await.insert(session_id.clone(), session);
        info!("Relay session created: {}", &session_id[..8]);

        // host_rx = messages destined FOR host
        // client_rx = messages destined FOR client
        Ok((host_rx, client_rx))
    }

    /// Forward data from host to client
    pub async fn host_to_client(&self, session_id: &str, data: Vec<u8>) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.bytes_relayed += data.len() as u64;
            session.client_tx.send(data).await.ok();
        }
        Ok(())
    }

    /// Forward data from client to host
    pub async fn client_to_host(&self, session_id: &str, data: Vec<u8>) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.bytes_relayed += data.len() as u64;
            session.host_tx.send(data).await.ok();
        }
        Ok(())
    }

    /// Remove relay session
    pub async fn remove_session(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().await.remove(session_id) {
            let duration = session.started_at.elapsed();
            let mb = session.bytes_relayed as f64 / 1024.0 / 1024.0;
            info!(
                "Relay session {} ended: {:.1}MB relayed in {:.0}s",
                &session_id[..8],
                mb,
                duration.as_secs_f64()
            );
        }
    }
}

