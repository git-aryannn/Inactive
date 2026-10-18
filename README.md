# Inactive — Ultra Low-Latency Encrypted Remote Desktop

An enterprise-grade, high-performance, self-hosted remote desktop software built in **Rust** and **Tauri**. Designed for cross-network operations (LAN + WAN / Global Internet) with direct P2P streaming, End-to-End Encryption (E2EE), and resilient system-service operation.

---

## 🌟 Key Features

- **Direct P2P Streaming via QUIC (UDP):** Minimum latency, zero cloud bandwidth cost when NAT traversal succeeds.
- **Automatic Fallback Relay (TURN):** Falls back to cloud relay when strict corporate firewalls or symmetric NATs prevent direct peer-to-peer connections.
- **Port 443 / HTTPS Compliant:** Traffic travels through port 443, making it indistinguishable from standard secure web traffic to pass restrictive firewalls.
- **End-to-End Encrypted (E2EE):** Per-session X25519 ECDH key exchange with ChaCha20-Poly1305 authenticated encryption. Even the cloud server cannot inspect screen frames or keystrokes.
- **Tamper & Termination Resistant:**
  - **Windows:** Runs as a native `SYSTEM`-level Windows Service (`InactiveAgent`). Protected from unauthorized termination by third-party tools.
  - **macOS:** Runs as a root `LaunchDaemon` (`/Library/LaunchDaemons/com.inactive.agent.plist`), surviving user logouts and automatically restarting upon crashes.
- **Durable & Persistent 9-Digit PIN:** Saved persistently to disk (`device_pin.json`). Does not expire or rotate on disconnections. Allows infinite seamless reconnects without needing a new PIN.
- **Hardware-Accelerated Screen Capture:**
  - Windows: DXGI Desktop Duplication API (60fps, low CPU overhead).
  - macOS: ScreenCaptureKit Metal-accelerated pipeline.
- **Clipboard Sync & Multi-Stream Multiplexing:** Independent streams for video, input control, and clipboard.

---

## 📐 Architecture

```
+-------------------------------------------------------------+
|                        Cloud Server                         |
|   Signaling Server (Axum/WebSocket :443) + STUN/TURN (:3478)|
+-------------------------------------------------------------+
              ^                                   ^
              | 1. Register PIN                   | 2. Connect with PIN
              |    & Public Key                   |    & Public Key
              v                                   v
+---------------------------+       +---------------------------+
|        Host Machine       |       |       Client Machine      |
|  (Windows/macOS Agent)    | <===> |    (Tauri / React App)    |
| - Screen Capture (DXGI/SCK)|  P2P  | - Video Render (Canvas)   |
| - Hardware Encoder        |  QUIC | - Keyboard & Mouse Input  |
| - Input Simulation (enigo)|       | - Clipboard Sync          |
+---------------------------+       +---------------------------+
```

---

## 📂 Project Structure

```
├── Cargo.toml                      # Root Cargo workspace
├── docker-compose.yml              # Cloud deployment (Signaling + coturn)
├── docker/
│   └── Dockerfile.signaling        # Multi-stage production container
├── crates/
│   ├── core/                       # Shared library
│   │   ├── src/capture/            # DXGI (Win) & ScreenCaptureKit (macOS)
│   │   ├── src/codec/              # Video frame compression & encoding
│   │   ├── src/crypto/             # E2EE (X25519 + ChaCha20-Poly1305)
│   │   ├── src/transport/          # QUIC connection & multiplexed streams
│   │   ├── src/input/              # Keyboard/mouse replay (enigo)
│   │   └── src/protocol/           # Message serialization
│   ├── host-agent/                 # Host daemon / service
│   │   ├── src/service_windows.rs  # Windows Service wrapper
│   │   ├── src/agent.rs            # Signaling loop & notifications
│   │   ├── src/session.rs          # Streaming & input worker tasks
│   │   └── src/pin.rs              # Durable 9-digit PIN management
│   ├── signaling-server/           # Cloud rendezvous server
│   │   ├── src/registry.rs         # In-memory session & PIN broker
│   │   ├── src/stun.rs             # STUN server implementation
│   │   └── src/relay.rs            # TURN relay fallback
│   └── client-app/                 # Tauri 2.0 Client UI (Inactive)
│       ├── src/                    # React 18 + TailwindCSS frontend
│       │   ├── components/ConnectScreen.tsx  # PIN Entry
│       │   ├── components/RemoteView.tsx     # Low-latency canvas renderer
│       │   └── components/Toolbar.tsx        # Session controls
│       └── src-tauri/              # Rust backend for Tauri app
└── installers/
    ├── macos/install.sh            # LaunchDaemon installer script
    └── windows/install.ps1         # Windows Service & Firewall setup script
```

---

## 🚀 Quick Start Guide

### 1. Prerequisites
- **Rust Toolchain:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js & npm:** Node 18+ (for client app UI)

### 2. Run Signaling Server (Local or Cloud)
```bash
cargo run --package signaling-server
# Server listens on 0.0.0.0:8080 (or port 443 with TLS)
```

### 3. Run Host Agent (Host Computer)
```bash
cargo run --package host-agent
# Agent outputs your durable 9-digit PIN and connects to signaling server
```

### 4. Run Client App (Controller Computer)
```bash
cd crates/client-app
npm install
npm run tauri dev
# Enter host's 9-digit PIN in the UI to connect!
```

---

## ☁️ Cloud Deployment (Production)

Deploy the signaling server and TURN relay on your VPS:

```bash
docker compose up -d
```

Ensure DNS records point your domain (e.g. `inactive.yourdomain.com`) to your server IP, and configure Let's Encrypt certificates in `./certs/`.

---

## 🔒 Security & E2EE Verification

- **Forward Secrecy:** Every connection generates an ephemeral ECDH keypair.
- **Zero-Knowledge Cloud:** The signaling server never receives session decryption keys.
- **Payload Authentication:** ChaCha20-Poly1305 provides AEAD verification against data tampering or injection.
