// crates/core/src/transport/mod.rs
// QUIC-based P2P transport layer
// Uses quinn crate (QUIC protocol with built-in TLS 1.3)
// Multiple streams: video (unreliable), control (reliable), file (reliable)

use anyhow::{Context, Result};
use quinn::{Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

/// Stream IDs for multiplexed QUIC streams
pub const STREAM_VIDEO: u8 = 0;    // Screen video frames
pub const STREAM_CONTROL: u8 = 1;  // Keyboard/mouse events, session control
pub const STREAM_FILE: u8 = 2;     // File transfer
pub const STREAM_CLIPBOARD: u8 = 3; // Clipboard sync

/// QUIC peer connection manager
pub struct QuicPeer {
    endpoint: Endpoint,
    connection: Option<Connection>,
    is_server: bool,
}

impl QuicPeer {
    /// Create a QUIC endpoint (server mode — host listens)
    pub async fn new_server(bind_addr: SocketAddr) -> Result<Self> {
        let (cert, key) = generate_self_signed_cert()?;

        let mut server_config = quinn::ServerConfig::with_single_cert(
            vec![cert],
            key,
        ).context("Failed to create server TLS config")?;

        // Optimize for low-latency streaming
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_idle_timeout(Some(
            std::time::Duration::from_secs(30).try_into()?
        ));
        transport_config.keep_alive_interval(Some(
            std::time::Duration::from_secs(5)
        ));
        server_config.transport_config(Arc::new(transport_config));

        let endpoint = Endpoint::server(server_config, bind_addr)
            .context("Failed to bind QUIC server endpoint")?;

        info!("QUIC server listening on {}", bind_addr);

        Ok(Self {
            endpoint,
            connection: None,
            is_server: true,
        })
    }

    /// Create a QUIC endpoint (client mode — connects to host)
    pub async fn new_client() -> Result<Self> {
        // Bind on any available port
        let endpoint = Endpoint::client("0.0.0.0:0".parse()?)
            .context("Failed to create QUIC client endpoint")?;

        Ok(Self {
            endpoint,
            connection: None,
            is_server: false,
        })
    }

    /// Accept incoming connection (server/host side)
    pub async fn accept(&mut self) -> Result<()> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .context("No incoming connection")?;

        let connection = incoming.await.context("Failed to accept QUIC connection")?;
        info!("Accepted connection from {}", connection.remote_address());
        self.connection = Some(connection);
        Ok(())
    }

    /// Connect to a peer (client side)
    pub async fn connect(&mut self, peer_addr: SocketAddr) -> Result<()> {
        // Accept any server certificate (E2EE key verification done separately)
        let crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?
        ));

        let connection = self
            .endpoint
            .connect_with(client_config, peer_addr, "inactive")?
            .await
            .context("Failed to connect to peer")?;

        info!("Connected to peer at {}", peer_addr);
        self.connection = Some(connection);
        Ok(())
    }

    /// Send encrypted video frame (opens uni-directional stream)
    pub async fn send_video_frame(&self, data: &[u8]) -> Result<()> {
        let conn = self.connection.as_ref().context("Not connected")?;
        let mut send = conn.open_uni().await?;
        // Stream type byte first
        send.write_u8(STREAM_VIDEO).await?;
        // Frame length prefix
        send.write_u32(data.len() as u32).await?;
        send.write_all(data).await?;
        send.finish()?;
        Ok(())
    }

    /// Send control event (keyboard/mouse/session)
    pub async fn send_control(&self, data: &[u8]) -> Result<()> {
        let conn = self.connection.as_ref().context("Not connected")?;
        let mut send = conn.open_uni().await?;
        send.write_u8(STREAM_CONTROL).await?;
        send.write_u32(data.len() as u32).await?;
        send.write_all(data).await?;
        send.finish()?;
        Ok(())
    }

    /// Receive next message (returns stream type + data)
    pub async fn receive(&self) -> Result<(u8, Vec<u8>)> {
        let conn = self.connection.as_ref().context("Not connected")?;
        let mut recv = conn.accept_uni().await?;

        // Read stream type
        let stream_type = recv.read_u8().await?;
        // Read length
        let len = recv.read_u32().await? as usize;
        // Read payload
        let mut data = vec![0u8; len];
        recv.read_exact(&mut data).await?;

        Ok((stream_type, data))
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection
            .as_ref()
            .map(|c| c.close_reason().is_none())
            .unwrap_or(false)
    }

    /// Gracefully close connection
    pub async fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close(0u32.into(), b"user_disconnect");
        }
    }
}

/// Generate self-signed TLS certificate for QUIC
/// (E2EE key verification done at application layer, not TLS layer)
fn generate_self_signed_cert() -> Result<(rustls::pki_types::CertificateDer<'static>, rustls::pki_types::PrivateKeyDer<'static>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["inactive".to_string()])
        .context("Failed to generate self-signed certificate")?;

    let cert_der = rustls::pki_types::CertificateDer::from(cert.cert);
    let key_der = rustls::pki_types::PrivateKeyDer::try_from(
        cert.key_pair.serialize_der()
    ).map_err(|e| anyhow::anyhow!("Key error: {}", e))?;

    Ok((cert_der, key_der))
}

/// Skip TLS server cert verification — we verify identity via E2EE keys instead
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes()
    }
}

