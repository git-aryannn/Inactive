// crates/core/src/crypto/mod.rs
// End-to-End Encryption using X25519 ECDH + ChaCha20-Poly1305
// Server sirf encrypted ciphertext dekhta hai — key kabhi server par nahi jata

use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

/// Per-session E2EE context
pub struct E2EESession {
    cipher: ChaCha20Poly1305,
    send_counter: u64,
    recv_counter: u64,
}

/// Result of ECDH handshake — own public key to share with peer
pub struct HandshakeResult {
    pub session: E2EESession,
    pub our_public_key: [u8; 32],
}

impl E2EESession {
    /// Step 1 (Initiator): Generate ephemeral keypair
    /// Returns: (our secret, our public key bytes to send to peer)
    pub fn generate_keypair() -> (EphemeralSecret, [u8; 32]) {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        (secret, public.to_bytes())
    }

    /// Step 2: Complete ECDH handshake with peer's public key
    /// Both sides call this with the OTHER side's public key
    /// Result: identical shared secret on both sides → same cipher key
    pub fn complete_handshake(
        our_secret: EphemeralSecret,
        their_public_key_bytes: [u8; 32],
    ) -> Result<Self> {
        let their_public_key = PublicKey::from(their_public_key_bytes);
        let shared_secret = our_secret.diffie_hellman(&their_public_key);

        // Derive a 256-bit key from the shared secret using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        hasher.update(b"remote-desktop-v1"); // domain separation
        let key_bytes = hasher.finalize();

        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);

        Ok(Self {
            cipher,
            send_counter: 0,
            recv_counter: 0,
        })
    }

    /// Encrypt a plaintext message
    /// Returns: (ciphertext, nonce) — both must be sent to peer
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12])> {
        // Unique nonce = counter (8 bytes) + random (4 bytes)
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..8].copy_from_slice(&self.send_counter.to_le_bytes());
        rand::thread_rng().fill_bytes(&mut nonce_bytes[8..]);
        self.send_counter += 1;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypt a received message
    pub fn decrypt(&mut self, ciphertext: &[u8], nonce_bytes: [u8; 12]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;
        self.recv_counter += 1;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_roundtrip() {
        // Simulate Alice and Bob
        let (alice_secret, alice_pub) = E2EESession::generate_keypair();
        let (bob_secret, bob_pub) = E2EESession::generate_keypair();

        // Both complete handshake with peer's public key
        let mut alice_session = E2EESession::complete_handshake(alice_secret, bob_pub).unwrap();
        let mut bob_session = E2EESession::complete_handshake(bob_secret, alice_pub).unwrap();

        // Alice encrypts
        let message = b"Hello from Alice!";
        let (ciphertext, nonce) = alice_session.encrypt(message).unwrap();

        // Bob decrypts
        let decrypted = bob_session.decrypt(&ciphertext, nonce).unwrap();
        assert_eq!(decrypted, message);

        // Make sure ciphertext is different from plaintext
        assert_ne!(ciphertext, message.to_vec());
    }
}

