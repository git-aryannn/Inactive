// crates/signaling-server/src/stun.rs
// Simple STUN server for NAT traversal
// Responds to STUN Binding Requests with the client's public IP:port
// This allows peers to discover their public-facing address for ICE

use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, warn};

const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;
const STUN_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// Run STUN server on UDP port 3478
pub async fn run_stun_server(bind_addr: SocketAddr) -> Result<()> {
    let socket = UdpSocket::bind(bind_addr).await?;
    info!("STUN server listening on {}", bind_addr);

    let mut buf = [0u8; 1024];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src_addr)) => {
                if let Err(e) = handle_stun_request(&socket, &buf[..len], src_addr).await {
                    warn!("STUN error from {}: {}", src_addr, e);
                }
            }
            Err(e) => {
                warn!("STUN recv error: {}", e);
            }
        }
    }
}

async fn handle_stun_request(
    socket: &UdpSocket,
    data: &[u8],
    src: SocketAddr,
) -> Result<()> {
    if data.len() < 20 {
        return Ok(());
    }

    // Parse STUN message type (first 2 bytes)
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != STUN_BINDING_REQUEST {
        return Ok(());
    }

    // Extract transaction ID (bytes 8-20)
    let mut transaction_id = [0u8; 12];
    transaction_id.copy_from_slice(&data[8..20]);

    // Build STUN Binding Response
    let response = build_binding_response(src, transaction_id);
    socket.send_to(&response, src).await?;

    Ok(())
}

fn build_binding_response(src: SocketAddr, transaction_id: [u8; 12]) -> Vec<u8> {
    let mut response = Vec::new();

    // XOR-Mapped-Address attribute
    let (ip_bytes, family) = match src {
        SocketAddr::V4(v4) => {
            let ip = u32::from(*v4.ip());
            let xor_ip = ip ^ STUN_MAGIC_COOKIE;
            (xor_ip.to_be_bytes().to_vec(), 0x01u8)
        }
        SocketAddr::V6(_) => {
            // IPv6 not implemented for simplicity
            return response;
        }
    };

    let port = src.port();
    let xor_port = port ^ (STUN_MAGIC_COOKIE >> 16) as u16;

    // XOR-Mapped-Address: family(1) + port(2) + ip(4) = 7 bytes + 1 pad = 8
    let attr_value = {
        let mut v = vec![0x00, family]; // padding + family
        v.extend_from_slice(&xor_port.to_be_bytes());
        v.extend_from_slice(&ip_bytes);
        v
    };

    // STUN header (20 bytes)
    response.extend_from_slice(&STUN_BINDING_RESPONSE.to_be_bytes()); // type
    let attr_len = (4 + attr_value.len()) as u16; // type(2) + len(2) + value
    response.extend_from_slice(&attr_len.to_be_bytes()); // message length
    response.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes()); // magic cookie
    response.extend_from_slice(&transaction_id); // transaction ID

    // XOR-Mapped-Address attribute
    response.extend_from_slice(&STUN_XOR_MAPPED_ADDRESS.to_be_bytes()); // attr type
    response.extend_from_slice(&(attr_value.len() as u16).to_be_bytes()); // attr length
    response.extend_from_slice(&attr_value);

    response
}

