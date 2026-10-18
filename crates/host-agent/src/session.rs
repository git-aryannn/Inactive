// crates/host-agent/src/session.rs
// Active remote desktop session — screen streaming + input handling

use anyhow::Result;
use remote_desktop_core::{
    capture::create_capture,
    input::InputSimulator,
    protocol::MessageType,
    transport::{QuicPeer, STREAM_CONTROL},
    E2EESession,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use x25519_dalek::EphemeralSecret;

/// Run a single remote session (screen capture + input replay)
pub async fn run_session(
    session_id: String,
    our_secret: EphemeralSecret,
    client_pub_key: [u8; 32],
    peer_addr: SocketAddr,
) -> Result<()> {
    info!("Session {} starting with peer {}", &session_id[..8], peer_addr);

    // Complete E2EE handshake
    let e2ee = Arc::new(Mutex::new(
        E2EESession::complete_handshake(our_secret, client_pub_key)?
    ));

    // Bind QUIC server (host listens, client connects)
    let bind_addr: SocketAddr = "0.0.0.0:0".parse()?; // Random available port
    let mut quic = QuicPeer::new_server(bind_addr).await?;

    info!("QUIC server bound, waiting for client connection...");
    quic.accept().await?;
    info!("Client connected via QUIC P2P!");

    // Initialize screen capture
    let capture = Arc::new(Mutex::new(create_capture()?));
    let screen_w = capture.lock().await.width();
    let screen_h = capture.lock().await.height();

    // Dedicated thread for InputSimulator (handles OS event loop & !Send pointers)
    let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<MessageType>(128);
    std::thread::spawn(move || {
        let mut sim = match InputSimulator::new(screen_w, screen_h) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to initialize InputSimulator: {}", e);
                return;
            }
        };
        while let Some(event) = input_rx.blocking_recv() {
            if let Err(e) = sim.handle_event(&event) {
                warn!("Input simulation error: {}", e);
            }
        }
    });

    // Clone Arc for tasks
    let quic = Arc::new(quic);
    let quic_clone = quic.clone();
    let e2ee_clone = e2ee.clone();

    // Task 1: Screen capture + encode + stream loop
    let screen_task = tokio::spawn(async move {
        let mut capture = capture.lock().await;
        // TODO: Initialize video encoder (ffmpeg H.264)
        let target_fps = 30u64;
        let frame_duration = tokio::time::Duration::from_millis(1000 / target_fps);

        loop {
            let tick = tokio::time::Instant::now();

            // Capture frame
            let frame = match capture.capture() {
                Ok(f) => f,
                Err(e) => {
                    error!("Capture error: {}", e);
                    break;
                }
            };

            // TODO: Encode frame with H.264 hardware encoder
            // For now, send raw (only for testing — too much bandwidth in production)
            let msg = MessageType::VideoFrame {
                data: frame.data,
                width: frame.width,
                height: frame.height,
                timestamp_ms: frame.timestamp_ms,
                keyframe: true,
            };

            // Serialize + encrypt
            let payload = bincode::serialize(&msg).unwrap_or_default();
            let encrypted = {
                let mut e = e2ee.lock().await;
                match e.encrypt(&payload) {
                    Ok((ct, nonce)) => {
                        let mut combined = nonce.to_vec();
                        combined.extend_from_slice(&ct);
                        combined
                    }
                    Err(e) => {
                        error!("Encrypt error: {}", e);
                        break;
                    }
                }
            };

            // Send over QUIC
            if let Err(e) = quic.send_video_frame(&encrypted).await {
                error!("Send error: {}", e);
                break;
            }

            // Maintain target FPS
            let elapsed = tick.elapsed();
            if elapsed < frame_duration {
                tokio::time::sleep(frame_duration - elapsed).await;
            }
        }
    });

    // Task 2: Receive input events from client
    let input_task = tokio::spawn(async move {
        loop {
            match quic_clone.receive().await {
                Ok((stream_type, data)) if stream_type == STREAM_CONTROL => {
                    // Decrypt
                    let decrypted = {
                        let mut e = e2ee_clone.lock().await;
                        if data.len() < 12 {
                            warn!("Short control message");
                            continue;
                        }
                        let mut nonce = [0u8; 12];
                        nonce.copy_from_slice(&data[..12]);
                        match e.decrypt(&data[12..], nonce) {
                            Ok(d) => d,
                            Err(e) => {
                                error!("Decrypt error: {}", e);
                                continue;
                            }
                        }
                    };

                    // Deserialize + forward to input simulator thread
                    if let Ok(event) = bincode::deserialize::<MessageType>(&decrypted) {
                        if let Err(e) = input_tx.send(event).await {
                            warn!("Failed to forward input event: {}", e);
                        }
                    }
                }
                Ok((stream_type, _)) => {
                    // Handle file transfer, clipboard etc
                    info!("Received stream type: {}", stream_type);
                }
                Err(e) => {
                    error!("Receive error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for either task to finish (means session ended)
    tokio::select! {
        _ = screen_task => info!("Screen task ended"),
        _ = input_task => info!("Input task ended"),
    }

    info!("Session {} ended", &session_id[..8]);
    Ok(())
}

