// crates/core/src/lib.rs
// Remote Desktop Core Library
// Shared between host-agent and client-app

pub mod capture;
pub mod codec;
pub mod crypto;
pub mod input;
pub mod protocol;
pub mod transport;

// Re-export main types
pub use protocol::{Message, MessageType, SessionInfo};
pub use crypto::E2EESession;
pub use transport::QuicPeer;

